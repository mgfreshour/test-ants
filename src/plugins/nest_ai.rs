use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{Ant, AntState, ColonyMember, Health, PlayerControlled};
use crate::components::map::{MapId, MapKind, MapMarker, MapPortal, PORTAL_RANGE};
use crate::components::nest::{
    AttendStep, Brood, BroodStage, CarriedBy, CellType, ChamberKind, DigStep, FeedStep, FoodEntity,
    HaulStep, MoveBroodStep, NestPath, NestTask, Queen, QueenHunger, StackedItem,
};
use crate::plugins::ant_ai::ColonyFood;
use crate::plugins::nest_navigation::{nest_grid_to_world, world_to_nest_grid, nest_grid_collision};
use crate::resources::active_map::{ActiveMap, MapRegistry};
use crate::resources::colony::BehaviorSliders;
use crate::resources::nest::{NestGrid, PlayerDigZones, TileStackRegistry, stack_position_offset, NEST_WIDTH};
use crate::resources::nest_pathfinding::NestPathCache;
use crate::resources::nest_pheromone::{
    NestPheromoneGrid, LABEL_BROOD, LABEL_ENTRANCE, LABEL_FOOD_STORAGE, LABEL_QUEEN,
};
use crate::resources::simulation::{SimClock, SimSpeed};
use crate::sim_core::regressions;
use crate::sim_core::nest_scoring::{choose_task, compute_scores, NestScoringInput, NestTaskChoice};
use crate::sim_core::nest_transitions;

pub struct NestAiPlugin;

impl Plugin for NestAiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_initial_nest_ants)
            .add_systems(
                Update,
                (
                    apply_brood_fed,
                    apply_brood_relocated,
                    apply_deferred_zone_expansions,
                    cleanup_orphaned_carried_items,
                    update_carried_item_positions,
                    apply_zone_expansions,
                    apply_excavated_cells,
                    portal_transition,
                    nest_to_surface_transition,
                    nest_ant_feeding,
                    nest_utility_scoring,
                    nest_task_advance,
                    construction_pheromone_deposit,
                    nest_separation_steering,
                    nest_grid_collision,
                    player_dig_zone_input,
                    nest_task_labels,
                )
                    .chain(),
            );
    }
}

/// Apply the BroodFed marker component to actually set brood.fed = true.
fn apply_brood_fed(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Brood), With<BroodFed>>,
) {
    for (entity, mut brood) in &mut query {
        brood.fed = true;
        commands.entity(entity).remove::<BroodFed>();
    }
}

/// Apply the BroodRelocated marker: set relocated = true and move brood to brood chamber.
fn apply_brood_relocated(
    mut commands: Commands,
    mut map_query: Query<(&NestGrid, &mut TileStackRegistry), With<MapMarker>>,
    registry: Res<MapRegistry>,
    mut query: Query<(Entity, &mut Brood, &mut Transform), With<BroodRelocated>>,
) {
    let Ok((grid, mut stack_registry)) = map_query.get_mut(registry.player_nest) else { return };

    for (entity, mut brood, mut transform) in &mut query {
        brood.relocated = true;

        let tile_pos = stack_registry
            .find_available_tile(&grid, ChamberKind::Brood)
            .or_else(|| {
                grid.find_expansion_candidate(ChamberKind::Brood).map(|exp| {
                    // Immediately expand the zone inline since we have mutable access
                    // The grid is immutable here, so we'll mark for deferred expansion
                    (exp.x, exp.y)
                })
            });

        if let Some(tile_pos) = tile_pos {
            // Check if this is an expansion (tile is currently a tunnel)
            if grid.get(tile_pos.0, tile_pos.1) == CellType::Tunnel {
                // Mark for expansion - will be processed by apply_zone_expansions
                commands.spawn(ExpandZoneDeferred {
                    x: tile_pos.0,
                    y: tile_pos.1,
                    chamber: ChamberKind::Brood,
                    map: registry.player_nest,
                });
            }

            if let Some(stack_idx) = stack_registry.push(tile_pos, entity) {
                let base_pos = nest_grid_to_world(tile_pos.0, tile_pos.1);
                let offset = stack_position_offset(stack_idx);

                transform.translation.x = base_pos.x + offset.x;
                transform.translation.y = base_pos.y + offset.y;

                commands.entity(entity).insert(StackedItem {
                    grid_pos: tile_pos,
                    stack_index: stack_idx,
                });
            }
        }

        commands.entity(entity).remove::<BroodRelocated>();
    }
}

/// Deferred zone expansion marker (spawned as entity, not component on ant).
#[derive(Component)]
struct ExpandZoneDeferred {
    x: usize,
    y: usize,
    chamber: ChamberKind,
    map: Entity,
}

/// Process deferred zone expansions from brood relocation.
fn apply_deferred_zone_expansions(
    mut commands: Commands,
    mut map_query: Query<(&mut NestGrid, &mut NestPathCache, &mut NestPheromoneGrid), With<MapMarker>>,
    query: Query<(Entity, &ExpandZoneDeferred)>,
    mut tile_query: Query<(&crate::components::nest::NestTile, &mut Sprite, &MapId)>,
) {
    use crate::resources::nest_pheromone::chamber_kind_to_label;

    for (entity, expand) in &query {
        let Ok((mut grid, mut path_cache, mut phero_grid)) = map_query.get_mut(expand.map) else {
            commands.entity(entity).despawn();
            continue;
        };

        let (x, y) = (expand.x, expand.y);
        let chamber = expand.chamber;

        if grid.get(x, y) == CellType::Tunnel {
            grid.set(x, y, CellType::Chamber(chamber));
            path_cache.invalidate();

            if let Some(phero) = phero_grid.get_mut(x, y) {
                let label_idx = chamber_kind_to_label(chamber);
                phero.chamber_labels[label_idx] = 1.0;
            }

            for (tile, mut sprite, tile_map_id) in &mut tile_query {
                if tile_map_id.0 == expand.map && tile.grid_x == x && tile.grid_y == y {
                    sprite.color = CellType::Chamber(chamber).color();
                    break;
                }
            }
        }
        commands.entity(entity).despawn();
    }
}

// ── Constants ──────────────────────────────────────────────────────────

/// How often nest ants re-evaluate their task (seconds).
const REEVALUATE_INTERVAL: f32 = 2.0;

/// Number of initial underground ants spawned at startup.
const INITIAL_NEST_ANTS: usize = 12;

// ── Spawn initial underground ants ────────────────────────────────────

fn spawn_initial_nest_ants(
    mut commands: Commands,
    registry: Res<MapRegistry>,
    map_query: Query<&NestGrid, With<MapMarker>>,
) {
    let Ok(grid) = map_query.get(registry.player_nest) else { return };
    let mut rng = rand::thread_rng();

    // Find passable cells for spawning
    let passable: Vec<(usize, usize)> = (0..grid.height)
        .flat_map(|y| (0..grid.width).map(move |x| (x, y)))
        .filter(|&(x, y)| grid.get(x, y).is_passable())
        .collect();

    if passable.is_empty() {
        return;
    }

    for _ in 0..INITIAL_NEST_ANTS {
        let &(gx, gy) = &passable[rng.gen_range(0..passable.len())];
        let pos = nest_grid_to_world(gx, gy);
        let jitter_x = rng.gen_range(-3.0..3.0);
        let jitter_y = rng.gen_range(-3.0..3.0);

        // Age varies — young ants nurse, older ants haul
        let age = rng.gen_range(0.0..300.0);

        commands.spawn((
            Sprite {
                color: Color::srgb(0.15, 0.12, 0.08),
                custom_size: Some(Vec2::splat(4.0)),
                ..default()
            },
            Transform::from_xyz(pos.x + jitter_x, pos.y + jitter_y, 2.5),
            Visibility::Hidden,
            Ant {
                caste: crate::components::ant::Caste::Worker,
                state: AntState::Nursing, // will be reassigned by utility AI
                age,
                hunger: 0.0,
            },
            Health::worker(),
            ColonyMember { colony_id: 0 },
            MapId(registry.player_nest),
            NestTask::Idle { timer: 0.0 },
        ));
    }
}

// ── Map Transitions ───────────────────────────────────────────────────

/// Generic portal transition: any ant within PORTAL_RANGE of a portal mouth on
/// its current map (and whose colony passes the restriction) transitions to the
/// target map at the target position. Replaces the old surface_to_nest_transition.
///
/// Per-colony throttling (the slider-based desired_underground count) is handled
/// by checking the portal's colony_id and the BehaviorSliders on the nest map.
fn portal_transition(
    clock: Res<SimClock>,
    sliders_query: Query<&BehaviorSliders, With<MapMarker>>,
    registry: Res<MapRegistry>,
    portal_query: Query<&MapPortal>,
    mut ant_query: Query<
        (Entity, &mut Transform, &mut Ant, &ColonyMember, &mut MapId, &mut Visibility),
        Without<PlayerControlled>,
    >,
    nest_task_query: Query<(), With<NestTask>>,
    mut commands: Commands,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    // Compute desired underground count from player nest's sliders.
    let total_ants = ant_query.iter().len();
    let current_underground = nest_task_query.iter().count();
    let desired_underground = if let Ok(sliders) = sliders_query.get(registry.player_nest) {
        ((sliders.nurse + sliders.dig) * total_ants as f32).ceil() as usize
    } else {
        0
    };

    let mut rng = rand::thread_rng();

    for (entity, mut transform, mut ant, colony, mut map_id, mut vis) in &mut ant_query {
        // Find all portals on the ant's current map that this colony can use.
        for portal in &portal_query {
            if portal.map != map_id.0 {
                continue;
            }
            if let Some(required_colony) = portal.colony_id {
                if colony.colony_id != required_colony {
                    continue;
                }
            }

            let pos = transform.translation.truncate();
            if pos.distance(portal.position) > PORTAL_RANGE {
                continue;
            }

            // Entering a nest — apply throttle based on sliders.
            let target_is_nest = portal.target_map != registry.surface;
            if target_is_nest {
                // Small random chance per frame to prevent all entering at once.
                if !regressions::should_enter_nest(
                    current_underground,
                    desired_underground,
                    ant.state == AntState::Foraging,
                    rng.gen::<f32>(),
                    0.02,
                ) {
                    break;
                }

                ant.state = AntState::Nursing;
                map_id.0 = portal.target_map;
                transform.translation.x = portal.target_position.x;
                transform.translation.y = portal.target_position.y;
                // Visibility will be corrected by sync_map_visibility.
                *vis = Visibility::Hidden;
                commands.entity(entity).insert(NestTask::Idle { timer: 0.0 });
            } else {
                // Exiting a nest — handled by nest_to_surface_transition when idle.
                // Portal exits are triggered there to preserve the idle-timeout logic.
            }
            break;
        }
    }
}

/// Nest ants that have been idle too long exit through a portal back to the surface.
/// Only older ants (age > 200) leave; younger ants stay underground longer.
fn nest_to_surface_transition(
    clock: Res<SimClock>,
    time: Res<Time>,
    registry: Res<MapRegistry>,
    portal_query: Query<&MapPortal>,
    mut commands: Commands,
    mut query: Query<
        (Entity, &mut Transform, &mut Ant, &mut NestTask, &mut MapId, &mut Visibility),
    >,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    let mut rng = rand::thread_rng();

    for (entity, mut transform, mut ant, mut task, mut map_id, mut vis) in &mut query {
        // Only process ants currently in a nest.
        if map_id.0 == registry.surface {
            continue;
        }

        if let NestTask::Idle { ref mut timer } = *task {
            *timer += dt;
            // Older ants (age > 200) exit after being idle for a few seconds.
            if ant.age > 200.0 && *timer > 5.0 {
                // Find an exit portal from this nest to the surface.
                let exit_portal = portal_query.iter().find(|p| {
                    p.map == map_id.0 && p.target_map == registry.surface
                });

                let surface_pos = if let Some(portal) = exit_portal {
                    portal.target_position
                } else {
                    // Fallback: stay put (no exit portal found).
                    continue;
                };

                ant.state = AntState::Foraging;
                map_id.0 = registry.surface;
                transform.translation.x = surface_pos.x + rng.gen_range(-15.0..15.0);
                transform.translation.y = surface_pos.y + rng.gen_range(-15.0..15.0);
                // Visibility will be corrected by sync_map_visibility.
                *vis = Visibility::Inherited;

                commands
                    .entity(entity)
                    .remove::<NestTask>()
                    .remove::<NestPath>();
            }
        }
    }
}

// ── Nest Ant Feeding ─────────────────────────────────────────────────

/// Nest ants eat from colony food stores when hungry. Without this they
/// have no way to reduce hunger and will starve.
const NEST_FEED_THRESHOLD: f32 = 0.4;
/// Hunger relief per feeding event.
const NEST_FEED_RELIEF: f32 = 0.5;
/// Colony food consumed per feeding event.
const NEST_FEED_COST: f32 = 0.2;

fn nest_ant_feeding(
    clock: Res<SimClock>,
    mut map_query: Query<&mut ColonyFood, With<MapMarker>>,
    mut query: Query<(&mut Ant, &MapId), With<NestTask>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    for (mut ant, map_id) in &mut query {
        if ant.hunger < NEST_FEED_THRESHOLD {
            continue;
        }

        let Ok(mut colony_food) = map_query.get_mut(map_id.0) else { continue };
        if colony_food.stored < NEST_FEED_COST {
            continue;
        }

        colony_food.stored -= NEST_FEED_COST;
        ant.hunger = (ant.hunger - NEST_FEED_RELIEF).max(0.0);
    }
}

// ── Utility AI Scoring ────────────────────────────────────────────────

/// Evaluate candidate actions for each nest ant and assign the best task.
/// Colony-agnostic: each ant reads data from its own nest map entity.
fn nest_utility_scoring(
    clock: Res<SimClock>,
    map_query: Query<(&NestGrid, &NestPheromoneGrid, &ColonyFood, &BehaviorSliders, Option<&PlayerDigZones>), With<MapMarker>>,
    brood_query: Query<(&Brood, &MapId)>,
    queen_query: Query<(&MapId, &QueenHunger), With<Queen>>,
    mut query: Query<(Entity, &Transform, &Ant, &MapId, &mut NestTask)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    // Pre-compute task counts per map from the query itself before mutating.
    let mut digger_counts: std::collections::HashMap<Entity, usize> = std::collections::HashMap::new();
    let mut mover_counts: std::collections::HashMap<Entity, usize> = std::collections::HashMap::new();
    let mut queen_counts: std::collections::HashMap<Entity, usize> = std::collections::HashMap::new();
    for (_, _, _, m, t) in query.iter() {
        if matches!(&*t, NestTask::Dig { .. }) {
            *digger_counts.entry(m.0).or_insert(0) += 1;
        }
        if matches!(&*t, NestTask::MoveBrood { .. }) {
            *mover_counts.entry(m.0).or_insert(0) += 1;
        }
        if matches!(&*t, NestTask::AttendQueen { .. }) {
            *queen_counts.entry(m.0).or_insert(0) += 1;
        }
    }

    for (_entity, transform, ant, map_id, mut task) in &mut query {
        // Only process ants on a nest map.
        let Ok((nest_grid, phero_grid, colony_food, _sliders, dig_zones_opt)) =
            map_query.get(map_id.0) else { continue };

        // Only re-evaluate when idle for long enough.
        // All tasks self-terminate to Idle when complete.
        let should_reevaluate = match &*task {
            NestTask::Idle { timer } => *timer > REEVALUATE_INTERVAL,
            _ => false,
        };

        if !should_reevaluate {
            continue;
        }

        let pos = transform.translation.truncate();
        let grid_pos = world_to_nest_grid(pos);

        // queen_hunger_val: 0 = fully fed, 1 = starving.
        let queen_data = queen_query
            .iter()
            .find(|(qmap, _)| qmap.0 == map_id.0);
        let has_queen = queen_data.is_some();
        let queen_hunger_val = queen_data
            .map(|(_, h)| 1.0 - h.satiation.clamp(0.0, 1.0))
            .unwrap_or(0.0);
        let unfed_larvae = brood_query
            .iter()
            .filter(|(b, m)| m.0 == map_id.0 && b.stage == BroodStage::Larva && !b.fed)
            .count();
        let unrelocated_brood = brood_query
            .iter()
            .filter(|(b, m)| m.0 == map_id.0 && !b.relocated)
            .count();

        let current_diggers = *digger_counts.get(&map_id.0).unwrap_or(&0);
        let current_movers = *mover_counts.get(&map_id.0).unwrap_or(&0);
        let current_queen_attendants = *queen_counts.get(&map_id.0).unwrap_or(&0);

        let dig_faces = nest_grid.find_dig_faces();
        let has_dig_faces = !dig_faces.is_empty();
        let brood_count = brood_query.iter().filter(|(_, m)| m.0 == map_id.0).count();
        let expansion_need = if brood_count > 8 { 0.3 } else { 0.0 };

        // Read pheromone inputs at current position.
        let (brood_need, queen_signal) = if let Some((gx, gy)) = grid_pos {
            let cell = phero_grid.get(gx, gy);
            (cell.brood_need, cell.queen_signal)
        } else {
            (0.0, 0.0)
        };

        let nearest_face_construction = if has_dig_faces {
            if let Some(gp) = grid_pos {
                dig_faces
                    .iter()
                    .min_by_key(|&&(fx, fy)| {
                        let dx = fx as i32 - gp.0 as i32;
                        let dy = fy as i32 - gp.1 as i32;
                        dx * dx + dy * dy
                    })
                    .map(|&(fx, fy)| phero_grid.get(fx, fy).construction)
                    .unwrap_or(0.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let scoring_input = NestScoringInput {
            unfed_larvae,
            unrelocated_brood,
            has_food: colony_food.stored > 0.5,
            colony_food_stored: colony_food.stored,
            has_queen,
            queen_hunger: queen_hunger_val,
            brood_need,
            queen_signal,
            nearest_face_construction,
            has_dig_faces,
            has_player_dig_zones: dig_zones_opt.map_or(false, |dz| !dz.cells.is_empty()),
            expansion_need,
            current_diggers,
            current_movers,
            current_queen_attendants,
            ant_age: ant.age,
        };
        let choice = choose_task(&compute_scores(&scoring_input));

        *task = match choice {
            NestTaskChoice::MoveBrood => NestTask::MoveBrood {
                step: MoveBroodStep::GoToQueen,
                target_brood: None,
            },
            NestTaskChoice::FeedLarva => NestTask::FeedLarva {
                step: FeedStep::GoToStorage,
                target_larva: None,
            },
            NestTaskChoice::Dig => NestTask::Dig {
                step: DigStep::GoToFace,
                target_cell: None,
                dig_timer: 0.0,
            },
            NestTaskChoice::HaulFood => NestTask::HaulFood {
                step: HaulStep::GoToEntrance,
            },
            NestTaskChoice::AttendQueen => NestTask::AttendQueen {
                step: AttendStep::GoToStorage,
            },
            NestTaskChoice::Idle => NestTask::Idle { timer: 0.0 },
        };
    }
}

// ── Task Chain Execution ──────────────────────────────────────────────

/// Advance task chain sub-steps: request pathfind, follow path, perform action.
/// Colony-agnostic: each ant uses resources from its own nest map entity.
fn nest_task_advance(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut map_query: Query<(&NestGrid, &NestPheromoneGrid, &mut NestPathCache, &mut ColonyFood, &mut TileStackRegistry, Option<&PlayerDigZones>), With<MapMarker>>,
    mut commands: Commands,
    brood_query: Query<(Entity, &Transform, &Brood)>,
    food_entity_query: Query<(Entity, &Transform, &FoodEntity, Option<&StackedItem>)>,
    carried_food_query: Query<(Entity, &CarriedBy), With<FoodEntity>>,
    mut queen_hunger_query: Query<&mut QueenHunger, With<Queen>>,
    mut ant_query: Query<(Entity, &Transform, &MapId, &mut NestTask, Option<&NestPath>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();

    // Pre-pass: count how many ants are already targeting each dig cell.
    let mut dig_target_counts: std::collections::HashMap<(usize, usize), usize> = std::collections::HashMap::new();
    for (_, _, _, task, _) in &ant_query {
        if let NestTask::Dig { target_cell: Some(cell), .. } = &*task {
            *dig_target_counts.entry(*cell).or_insert(0) += 1;
        }
    }

    for (entity, transform, map_id, mut task, path) in &mut ant_query {
        // Only process ants on a nest map (one with NestGrid).
        let Ok((grid, phero_grid, mut path_cache, mut colony_food, mut stack_registry, dig_zones_opt)) =
            map_query.get_mut(map_id.0) else { continue };

        let pos = transform.translation.truncate();
        let grid_pos = match world_to_nest_grid(pos) {
            Some(gp) => gp,
            None => continue,
        };

        // path_done = true only when an actual path was followed to completion.
        // has_no_path = true when the ant has no NestPath component at all.
        let path_done = path.map_or(false, |p| p.is_complete());
        let has_no_path = path.is_none();

        // "at_destination" = ant has arrived (path completed) or has no path (already cleaned up).
        // Used by action steps that need the ant to be at a location.
        let at_destination = nest_transitions::at_destination(path_done, has_no_path);

        match &mut *task {
            NestTask::FeedLarva { step, target_larva } => {
                match step {
                    FeedStep::GoToStorage => {
                        if at_destination {
                            if let Some(next) =
                                nest_transitions::next_feed_step_on_arrival(*step, path.is_some())
                            {
                                *step = next;
                            } else {
                                // No path yet — request one.
                                if let Some(goal) = find_label_cell(&grid, LABEL_FOOD_STORAGE) {
                                    if let Some(waypoints) = path_cache.find_path(&grid, grid_pos, goal) {
                                        commands.entity(entity).insert(NestPath::new(waypoints));
                                    } else {
                                        *task = NestTask::Idle { timer: 0.0 };
                                        continue;
                                    }
                                } else {
                                    *task = NestTask::Idle { timer: 0.0 };
                                    continue;
                                }
                            }
                        }
                    }
                    FeedStep::PickUpFood => {
                        if at_destination {
                            // Find nearest stacked food in storage
                            let storage_food = food_entity_query
                                .iter()
                                .filter(|(_, _, _, stacked_opt)| {
                                    stacked_opt.map_or(false, |s| {
                                        grid.get(s.grid_pos.0, s.grid_pos.1) == CellType::Chamber(ChamberKind::FoodStorage)
                                    })
                                })
                                .min_by_key(|(_, food_tf, _, _)| {
                                    (pos.distance(food_tf.translation.truncate()) * 100.0) as i32
                                });

                            if let Some((food_e, _, _, stacked_opt)) = storage_food {
                                if let Some(stacked) = stacked_opt {
                                    stack_registry.remove(stacked.grid_pos, food_e);
                                }
                                commands.entity(food_e).remove::<StackedItem>();
                                commands.entity(food_e).insert(CarriedBy(entity));
                                commands.entity(entity).remove::<NestPath>();
                                *step = FeedStep::GoToBrood;
                            } else {
                                // No food available, go idle.
                                *task = NestTask::Idle { timer: 0.0 };
                                continue;
                            }
                        }
                    }
                    FeedStep::GoToBrood => {
                        if at_destination {
                            if let Some(next) =
                                nest_transitions::next_feed_step_on_arrival(*step, path.is_some())
                            {
                                *step = next;
                            } else {
                                if let Some(goal) = find_label_cell(&grid, LABEL_BROOD) {
                                    if let Some(waypoints) = path_cache.find_path(&grid, grid_pos, goal) {
                                        commands.entity(entity).insert(NestPath::new(waypoints));
                                    } else {
                                        *task = NestTask::Idle { timer: 0.0 };
                                        continue;
                                    }
                                } else {
                                    *task = NestTask::Idle { timer: 0.0 };
                                    continue;
                                }
                            }
                        }
                    }
                    FeedStep::FindLarva => {
                        if at_destination {
                            // Find nearest unfed larva.
                            let mut best: Option<(Entity, f32)> = None;
                            for (brood_entity, brood_tf, brood) in &brood_query {
                                if brood.stage != BroodStage::Larva || brood.fed {
                                    continue;
                                }
                                let dist = pos.distance(brood_tf.translation.truncate());
                                if best.is_none() || dist < best.unwrap().1 {
                                    best = Some((brood_entity, dist));
                                }
                            }
                            if let Some((larva_entity, _)) = best {
                                *target_larva = Some(larva_entity);
                                *step = FeedStep::DeliverFood;
                            } else {
                                // No unfed larvae found, task complete.
                                *step = FeedStep::DeliverFood;
                            }
                        }
                    }
                    FeedStep::DeliverFood => {
                        // Feed the larva.
                        if let Some(larva_entity) = target_larva {
                            commands.entity(*larva_entity).try_insert(BroodFed);
                        }

                        // Despawn carried food entity
                        for (food_e, carried_by) in &carried_food_query {
                            if carried_by.0 == entity {
                                commands.entity(food_e).despawn();
                                colony_food.stored -= 1.0;
                                break;
                            }
                        }

                        *task = NestTask::Idle { timer: 0.0 };
                        continue;
                    }
                }
            }

            NestTask::MoveBrood { step, target_brood } => {
                match step {
                    MoveBroodStep::GoToQueen => {
                        if at_destination {
                            if let Some(next) = nest_transitions::next_move_brood_step_on_arrival(
                                *step,
                                path.is_some(),
                            ) {
                                *step = next;
                            } else {
                                if let Some(goal) = find_label_cell(&grid, LABEL_QUEEN) {
                                    if let Some(waypoints) = path_cache.find_path(&grid, grid_pos, goal) {
                                        commands.entity(entity).insert(NestPath::new(waypoints));
                                    } else {
                                        *task = NestTask::Idle { timer: 0.0 };
                                        continue;
                                    }
                                } else {
                                    *task = NestTask::Idle { timer: 0.0 };
                                    continue;
                                }
                            }
                        }
                    }
                    MoveBroodStep::PickUpBrood => {
                        if at_destination {
                            // Find nearest unrelocated brood.
                            let mut best: Option<(Entity, f32)> = None;
                            for (brood_entity, brood_tf, brood) in &brood_query {
                                if brood.relocated {
                                    continue;
                                }
                                let dist = pos.distance(brood_tf.translation.truncate());
                                if best.is_none() || dist < best.unwrap().1 {
                                    best = Some((brood_entity, dist));
                                }
                            }
                            if let Some((brood_entity, _)) = best {
                                *target_brood = Some(brood_entity);
                                // Attach brood to ant so it follows.
                                commands.entity(brood_entity).insert(CarriedBy(entity));
                                commands.entity(entity).remove::<NestPath>();
                                *step = MoveBroodStep::GoToBrood;
                            } else {
                                *task = NestTask::Idle { timer: 0.0 };
                                continue;
                            }
                        }
                    }
                    MoveBroodStep::GoToBrood => {
                        if at_destination {
                            if let Some(next) = nest_transitions::next_move_brood_step_on_arrival(
                                *step,
                                path.is_some(),
                            ) {
                                *step = next;
                            } else {
                                if let Some(goal) = find_label_cell(&grid, LABEL_BROOD) {
                                    if let Some(waypoints) = path_cache.find_path(&grid, grid_pos, goal) {
                                        commands.entity(entity).insert(NestPath::new(waypoints));
                                    } else {
                                        *task = NestTask::Idle { timer: 0.0 };
                                        continue;
                                    }
                                } else {
                                    *task = NestTask::Idle { timer: 0.0 };
                                    continue;
                                }
                            }
                        }
                    }
                    MoveBroodStep::PlaceBrood => {
                        if at_destination {
                            if let Some(brood_entity) = target_brood {
                                // Release brood from ant, then mark for relocation.
                                commands.entity(*brood_entity).remove::<CarriedBy>();
                                commands.entity(*brood_entity).try_insert(BroodRelocated);
                            }
                            *task = NestTask::Idle { timer: 0.0 };
                            continue;
                        }
                    }
                }
            }

            NestTask::HaulFood { step } => {
                match step {
                    HaulStep::GoToEntrance => {
                        if at_destination {
                            if let Some(next) =
                                nest_transitions::next_haul_step_on_arrival(*step, path.is_some())
                            {
                                *step = next;
                            } else {
                                if let Some(goal) = find_label_cell(&grid, LABEL_ENTRANCE) {
                                    if let Some(waypoints) = path_cache.find_path(&grid, grid_pos, goal) {
                                        commands.entity(entity).insert(NestPath::new(waypoints));
                                    } else {
                                        *task = NestTask::Idle { timer: 0.0 };
                                        continue;
                                    }
                                } else {
                                    *task = NestTask::Idle { timer: 0.0 };
                                    continue;
                                }
                            }
                        }
                    }
                    HaulStep::PickUpFood => {
                        if at_destination {
                            // Find nearest non-stacked food entity at entrance
                            let mut best: Option<(Entity, f32)> = None;
                            for (food_e, food_tf, _, stacked_opt) in &food_entity_query {
                                // Only pick up food that isn't already stacked
                                if stacked_opt.is_some() {
                                    continue;
                                }
                                let food_pos = food_tf.translation.truncate();
                                if let Some((gx, gy)) = world_to_nest_grid(food_pos) {
                                    let cx = grid.width / 2;
                                    // Entrance tunnel: column cx, rows 0-6
                                    if gx == cx && gy <= 6 {
                                        let dist = pos.distance(food_pos);
                                        if best.is_none() || dist < best.unwrap().1 {
                                            best = Some((food_e, dist));
                                        }
                                    }
                                }
                            }

                            if let Some((food_entity, _)) = best {
                                commands.entity(food_entity).insert(CarriedBy(entity));
                                commands.entity(entity).remove::<NestPath>();
                                *step = HaulStep::GoToStorage;
                            } else {
                                *task = NestTask::Idle { timer: 0.0 };
                                continue;
                            }
                        }
                    }
                    HaulStep::GoToStorage => {
                        if at_destination {
                            if let Some(next) =
                                nest_transitions::next_haul_step_on_arrival(*step, path.is_some())
                            {
                                *step = next;
                            } else {
                                if let Some(goal) = find_label_cell(&grid, LABEL_FOOD_STORAGE) {
                                    if let Some(waypoints) = path_cache.find_path(&grid, grid_pos, goal) {
                                        commands.entity(entity).insert(NestPath::new(waypoints));
                                    } else {
                                        *task = NestTask::Idle { timer: 0.0 };
                                        continue;
                                    }
                                } else {
                                    *task = NestTask::Idle { timer: 0.0 };
                                    continue;
                                }
                            }
                        }
                    }
                    HaulStep::DropFood => {
                        if at_destination {
                            // Find carried food entity
                            let carried_food = carried_food_query
                                .iter()
                                .find(|(_, carried_by)| carried_by.0 == entity);

                            if let Some((food_e, _)) = carried_food {
                                // Stack in food storage chamber
                                let tile_pos = stack_registry
                                    .find_available_tile(&grid, ChamberKind::FoodStorage)
                                    .or_else(|| {
                                        grid.find_expansion_candidate(ChamberKind::FoodStorage)
                                            .map(|exp| {
                                                commands.entity(entity).insert(ExpandZone {
                                                    x: exp.x,
                                                    y: exp.y,
                                                    chamber: ChamberKind::FoodStorage,
                                                });
                                                (exp.x, exp.y)
                                            })
                                    });

                                if let Some(tile_pos) = tile_pos {
                                    if let Some(stack_idx) = stack_registry.push(tile_pos, food_e) {
                                        let base_pos = nest_grid_to_world(tile_pos.0, tile_pos.1);
                                        let offset = stack_position_offset(stack_idx);

                                        commands.entity(food_e).remove::<CarriedBy>();
                                        commands.entity(food_e).insert((
                                            StackedItem { grid_pos: tile_pos, stack_index: stack_idx },
                                            Transform::from_xyz(base_pos.x + offset.x, base_pos.y + offset.y, 2.5),
                                        ));
                                    } else {
                                        // Stack full, drop at current location
                                        commands.entity(food_e).remove::<CarriedBy>();
                                    }
                                } else {
                                    // No available tiles and no expansion possible, drop at current location
                                    commands.entity(food_e).remove::<CarriedBy>();
                                }
                            }
                            *task = NestTask::Idle { timer: 0.0 };
                            continue;
                        }
                    }
                }
            }

            NestTask::AttendQueen { step } => {
                match step {
                    AttendStep::GoToStorage => {
                        if at_destination {
                            if let Some(next) = nest_transitions::next_attend_step_on_arrival(
                                *step,
                                path.is_some(),
                            ) {
                                *step = next;
                            } else {
                                if let Some(goal) = find_label_cell(&grid, LABEL_FOOD_STORAGE) {
                                    if let Some(waypoints) = path_cache.find_path(&grid, grid_pos, goal) {
                                        commands.entity(entity).insert(NestPath::new(waypoints));
                                    } else {
                                        *task = NestTask::Idle { timer: 0.0 };
                                        continue;
                                    }
                                } else {
                                    *task = NestTask::Idle { timer: 0.0 };
                                    continue;
                                }
                            }
                        }
                    }
                    AttendStep::PickUpFood => {
                        if at_destination {
                            // Find nearest stacked food in storage.
                            let storage_food = food_entity_query
                                .iter()
                                .filter(|(_, _, _, stacked_opt)| {
                                    stacked_opt.map_or(false, |s| {
                                        grid.get(s.grid_pos.0, s.grid_pos.1) == CellType::Chamber(ChamberKind::FoodStorage)
                                    })
                                })
                                .min_by_key(|(_, food_tf, _, _)| {
                                    (pos.distance(food_tf.translation.truncate()) * 100.0) as i32
                                });

                            if let Some((food_e, _, _, stacked_opt)) = storage_food {
                                if let Some(stacked) = stacked_opt {
                                    stack_registry.remove(stacked.grid_pos, food_e);
                                }
                                commands.entity(food_e).remove::<StackedItem>();
                                commands.entity(food_e).insert(CarriedBy(entity));
                                commands.entity(entity).remove::<NestPath>();
                                *step = AttendStep::GoToQueen;
                            } else {
                                // No food available, go idle.
                                *task = NestTask::Idle { timer: 0.0 };
                                continue;
                            }
                        }
                    }
                    AttendStep::GoToQueen => {
                        if at_destination {
                            if let Some(next) = nest_transitions::next_attend_step_on_arrival(
                                *step,
                                path.is_some(),
                            ) {
                                *step = next;
                            } else {
                                if let Some(goal) = find_label_cell(&grid, LABEL_QUEEN) {
                                    if let Some(waypoints) = path_cache.find_path(&grid, grid_pos, goal) {
                                        commands.entity(entity).insert(NestPath::new(waypoints));
                                    } else {
                                        *task = NestTask::Idle { timer: 0.0 };
                                        continue;
                                    }
                                } else {
                                    *task = NestTask::Idle { timer: 0.0 };
                                    continue;
                                }
                            }
                        }
                    }
                    AttendStep::FeedQueen => {
                        if at_destination {
                            // Find our carried food (may have been lost to a race condition).
                            let carried = carried_food_query
                                .iter()
                                .find(|(_, cb)| cb.0 == entity);

                            if let Some((food_e, _)) = carried {
                                // Transfer food to queen — boost her satiation.
                                if let Ok(mut hunger) = queen_hunger_query.get_single_mut() {
                                    hunger.satiation = (hunger.satiation + 1.0).min(2.0);
                                }
                                commands.entity(food_e).despawn();
                                colony_food.stored -= 1.0;
                            }

                            *task = NestTask::Idle { timer: 0.0 };
                            continue;
                        }
                    }
                }
            }

            NestTask::Dig { step, target_cell, dig_timer } => {
                let dig_zones_cells = dig_zones_opt.map(|dz| &dz.cells);

                match step {
                    DigStep::GoToFace => {
                        if at_destination {
                            if let Some(next) =
                                nest_transitions::next_dig_step_on_arrival(*step, path.is_some())
                            {
                                // Path existed and completed — advance to excavation.
                                *step = next;
                            } else {
                                // No path yet — pick a target and request path.
                                let dig_faces = grid.find_dig_faces();
                                if dig_faces.is_empty() {
                                    *task = NestTask::Idle { timer: 0.0 };
                                    continue;
                                }
                                let mut rng = rand::thread_rng();
                                // Filter out faces with 5+ ants already targeting them.
                                let available_faces =
                                    regressions::select_available_dig_faces(
                                        &dig_faces,
                                        &dig_target_counts,
                                        5,
                                    );
                                let faces_to_score = &available_faces;
                                let mut scored: Vec<((usize, usize), f32)> = faces_to_score
                                    .iter()
                                    .map(|&(fx, fy)| {
                                        let construction = phero_grid.get(fx, fy).construction;
                                        let player_bonus = dig_zones_cells
                                            .map_or(false, |cells| cells.contains(&(fx, fy)))
                                            .then_some(0.5)
                                            .unwrap_or(0.0);
                                        let dx = fx as i32 - grid_pos.0 as i32;
                                        let dy = fy as i32 - grid_pos.1 as i32;
                                        let dist_sq = (dx * dx + dy * dy) as f32;
                                        let proximity = 1.0 / (1.0 + dist_sq.sqrt() * 0.1);

                                        let solid_neighbors = [(-1i32, 0), (1, 0), (0, -1i32), (0, 1)]
                                            .iter()
                                            .filter(|&&(ndx, ndy)| {
                                                let nx = fx as i32 + ndx;
                                                let ny = fy as i32 + ndy;
                                                nx >= 0
                                                    && ny >= 0
                                                    && (nx as usize) < grid.width
                                                    && (ny as usize) < grid.height
                                                    && !grid.get(nx as usize, ny as usize).is_passable()
                                            })
                                            .count();
                                        let narrowness = match solid_neighbors {
                                            3 => 1.0,
                                            2 => 0.6,
                                            1 => 0.3,
                                            _ => 0.1,
                                        };

                                        let score = (construction + player_bonus + 0.1) * proximity * narrowness;
                                        let jitter = rng.gen_range(0.0..0.15);
                                        ((fx, fy), score + jitter)
                                    })
                                    .collect();
                                scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                                let best = scored.first().map(|&(pos, _)| pos);
                                if let Some(face) = best {
                                    *target_cell = Some(face);
                                    let adjacent = find_adjacent_passable(&grid, face.0, face.1);
                                    if let Some(adj) = adjacent {
                                        if let Some(waypoints) = path_cache.find_path(&grid, grid_pos, adj) {
                                            commands.entity(entity).insert(NestPath::new(waypoints));
                                        } else {
                                            *task = NestTask::Idle { timer: 0.0 };
                                            continue;
                                        }
                                    } else {
                                        *task = NestTask::Idle { timer: 0.0 };
                                        continue;
                                    }
                                } else {
                                    *task = NestTask::Idle { timer: 0.0 };
                                    continue;
                                }
                            }
                        }
                    }
                    DigStep::Excavate => {
                        if at_destination {
                            if let Some((tx, ty)) = *target_cell {
                                let cell = grid.get(tx, ty);
                                if !cell.is_diggable() {
                                    // Target was already dug by another ant — pick a new face.
                                    *dig_timer = 0.0;
                                    *target_cell = None;
                                    commands.entity(entity).remove::<NestPath>();
                                    *step = DigStep::GoToFace;
                                    continue;
                                }
                                let duration = cell.dig_duration();
                                *dig_timer += dt;
                                if *dig_timer >= duration {
                                    *dig_timer = 0.0;
                                    // Mark cell for excavation via marker component.
                                    commands.entity(entity).insert(ExcavatedCell { x: tx, y: ty });
                                    // Soil magically disappears — immediately look for next face.
                                    *target_cell = None;
                                    commands.entity(entity).remove::<NestPath>();
                                    *step = DigStep::GoToFace;
                                    continue;
                                }
                            } else {
                                *task = NestTask::Idle { timer: 0.0 };
                                continue;
                            }
                        }
                    }
                    // PickUpSoil, GoToMidden, DropSoil unused — soil vanishes on excavation.
                    _ => {
                        *task = NestTask::Idle { timer: 0.0 };
                        continue;
                    }
                }
            }

            NestTask::Idle { timer } => {
                *timer += dt;
            }
        }
    }
}

/// Temporary marker to feed brood (since we can't mutate Brood in the same query).
#[derive(Component)]
struct BroodFed;

/// Temporary marker: brood has been relocated to the brood chamber.
#[derive(Component)]
struct BroodRelocated;

/// Update carried item positions to follow the ant carrying them.
fn update_carried_item_positions(
    ant_query: Query<(Entity, &Transform), With<NestTask>>,
    mut item_query: Query<(&mut Transform, &CarriedBy), (Or<(With<Brood>, With<FoodEntity>)>, Without<NestTask>)>,
) {
    for (mut item_tf, carried_by) in &mut item_query {
        if let Ok((_, ant_tf)) = ant_query.get(carried_by.0) {
            // Position item at ant's location with slight offset.
            item_tf.translation.x = ant_tf.translation.x;
            item_tf.translation.y = ant_tf.translation.y + 3.0;
        }
    }
}

fn cleanup_orphaned_carried_items(
    mut commands: Commands,
    item_query: Query<(Entity, &CarriedBy), Or<(With<FoodEntity>, With<Brood>)>>,
    ant_query: Query<(), With<NestTask>>,
) {
    for (item_entity, carried_by) in &item_query {
        if ant_query.get(carried_by.0).is_err() {
            commands.entity(item_entity).remove::<CarriedBy>();
        }
    }
}

/// Marker component: an ant has excavated a cell and the grid should be updated.
#[derive(Component)]
struct ExcavatedCell {
    x: usize,
    y: usize,
}

/// Marker component: a tunnel cell should be converted to a chamber to expand a zone.
#[derive(Component)]
struct ExpandZone {
    x: usize,
    y: usize,
    chamber: ChamberKind,
}

// ── Zone Expansion ───────────────────────────────────────────────────

/// Process ExpandZone markers: convert tunnel cells to chambers, update sprites and pheromones.
fn apply_zone_expansions(
    mut commands: Commands,
    mut map_query: Query<(&mut NestGrid, &mut NestPathCache, &mut NestPheromoneGrid), With<MapMarker>>,
    mut query: Query<(Entity, &ExpandZone, &MapId)>,
    mut tile_query: Query<(&crate::components::nest::NestTile, &mut Sprite, &MapId), Without<ExpandZone>>,
) {
    use crate::resources::nest_pheromone::chamber_kind_to_label;

    for (entity, expand, map_id) in &mut query {
        let Ok((mut grid, mut path_cache, mut phero_grid)) = map_query.get_mut(map_id.0) else {
            commands.entity(entity).remove::<ExpandZone>();
            continue;
        };

        let (x, y) = (expand.x, expand.y);
        let chamber = expand.chamber;

        if grid.get(x, y) == CellType::Tunnel {
            grid.set(x, y, CellType::Chamber(chamber));
            path_cache.invalidate();

            if let Some(phero) = phero_grid.get_mut(x, y) {
                let label_idx = chamber_kind_to_label(chamber);
                phero.chamber_labels[label_idx] = 1.0;
            }

            for (tile, mut sprite, tile_map_id) in &mut tile_query {
                if tile_map_id.0 == map_id.0 && tile.grid_x == x && tile.grid_y == y {
                    sprite.color = CellType::Chamber(chamber).color();
                    break;
                }
            }
        }
        commands.entity(entity).remove::<ExpandZone>();
    }
}

// ── Excavation Grid Mutation ─────────────────────────────────────────

/// Process ExcavatedCell markers: mutate the NestGrid, invalidate path cache,
/// and update the tile sprite so the player sees the newly dug tunnel.
fn apply_excavated_cells(
    mut commands: Commands,
    mut map_query: Query<(&mut NestGrid, &mut NestPathCache), With<MapMarker>>,
    mut query: Query<(Entity, &ExcavatedCell, &MapId)>,
    mut tile_query: Query<(&crate::components::nest::NestTile, &mut Sprite, &MapId), Without<ExcavatedCell>>,
) {
    for (entity, excavated, map_id) in &mut query {
        let Ok((mut grid, mut path_cache)) = map_query.get_mut(map_id.0) else {
            commands.entity(entity).remove::<ExcavatedCell>();
            continue;
        };

        let (x, y) = (excavated.x, excavated.y);
        if grid.get(x, y).is_diggable() {
            grid.set(x, y, CellType::Tunnel);
            path_cache.invalidate();

            // Update the tile sprite color to match the new cell type.
            for (tile, mut sprite, tile_map_id) in &mut tile_query {
                if tile_map_id.0 == map_id.0 && tile.grid_x == x && tile.grid_y == y {
                    sprite.color = CellType::Tunnel.color();
                    break;
                }
            }
        }
        commands.entity(entity).remove::<ExcavatedCell>();
    }
}

// ── Construction Pheromone ───────────────────────────────────────────

/// Diggers deposit construction pheromone at their target dig face.
/// Self-limiting: pheromone concentration caps and nearby crowding dampens deposit.
fn construction_pheromone_deposit(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut map_query: Query<&mut NestPheromoneGrid, With<MapMarker>>,
    query: Query<(&Transform, &NestTask, &MapId)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    let deposit_rate = 0.15; // per second
    let max_construction = 1.0;

    for (transform, task, map_id) in &query {
        let Ok(mut phero_grid) = map_query.get_mut(map_id.0) else { continue };

        if let NestTask::Dig { step, target_cell, .. } = task {
            // Only deposit while actively excavating or approaching dig face.
            if *step != DigStep::Excavate && *step != DigStep::GoToFace {
                continue;
            }
            if let Some((tx, ty)) = target_cell {
                if let Some(cell) = phero_grid.get_mut(*tx, *ty) {
                    // Self-limiting: deposit less when concentration is already high.
                    let headroom = (max_construction - cell.construction).max(0.0);
                    cell.construction += deposit_rate * dt * headroom;
                    cell.construction = cell.construction.min(max_construction);
                }

                // Also deposit lightly on the ant's current position.
                let pos = transform.translation.truncate();
                if let Some((gx, gy)) = world_to_nest_grid(pos) {
                    if let Some(cell) = phero_grid.get_mut(gx, gy) {
                        let headroom = (max_construction - cell.construction).max(0.0);
                        cell.construction += deposit_rate * 0.3 * dt * headroom;
                        cell.construction = cell.construction.min(max_construction);
                    }
                }
            }
        }
    }
}

// ── Separation Steering ─────────────────────────────────────────────

/// Gentle push-apart force for nest ants to prevent clumping in tunnels.
fn nest_separation_steering(
    clock: Res<SimClock>,
    time: Res<Time>,
    map_query: Query<&NestGrid, With<MapMarker>>,
    mut query: Query<(Entity, &mut Transform, &MapId), With<NestTask>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    let separation_radius = 8.0f32;
    let separation_strength = 30.0f32;

    // Collect positions first to avoid borrow conflicts.
    let positions: Vec<(Entity, Vec2, Entity)> = query
        .iter()
        .map(|(e, t, m)| (e, t.translation.truncate(), m.0))
        .collect();

    for (entity, mut transform, map_id) in &mut query {
        let Ok(grid) = map_query.get(map_id.0) else { continue };

        let pos = transform.translation.truncate();
        let mut push = Vec2::ZERO;

        for &(other_entity, other_pos, other_map) in &positions {
            // Only push against ants on the same map.
            if other_entity == entity || other_map != map_id.0 {
                continue;
            }
            let diff = pos - other_pos;
            let dist = diff.length();
            if dist > 0.1 && dist < separation_radius {
                // Inverse-distance push.
                let force = diff.normalize() * (1.0 - dist / separation_radius);
                push += force;
            }
        }

        if push.length() > 0.01 {
            let displacement = push.normalize() * separation_strength * dt;
            let new_pos = pos + displacement;

            // Only apply if new position is still in a passable cell.
            if let Some((gx, gy)) = world_to_nest_grid(new_pos) {
                if grid.get(gx, gy).is_passable() {
                    transform.translation.x = new_pos.x;
                    transform.translation.y = new_pos.y;
                }
            }
        }
    }
}

// ── Player Dig Zone Input ───────────────────────────────────────────

/// In underground view, left-click to designate dig zones, right-click to clear.
fn player_dig_zone_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform), With<crate::plugins::camera::MainCamera>>,
    active: Res<ActiveMap>,
    mut map_query: Query<(&NestGrid, &mut PlayerDigZones), With<MapMarker>>,
    mut tile_query: Query<(&crate::components::nest::NestTile, &mut Sprite, &MapId)>,
) {
    // Only process when viewing a nest.
    if !matches!(active.kind, MapKind::Nest { .. }) {
        return;
    }

    let left = mouse.just_pressed(MouseButton::Left);
    let right = mouse.just_pressed(MouseButton::Right);
    if !left && !right {
        return;
    }

    let Ok(window) = windows.get_single() else { return };
    let Some(cursor_pos) = window.cursor_position() else { return };
    let Ok((camera, cam_transform)) = camera_query.get_single() else { return };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else { return };

    let Some((gx, gy)) = world_to_nest_grid(world_pos) else { return };

    let Ok((grid, mut dig_zones)) = map_query.get_mut(active.entity) else { return };

    if left {
        // Only allow marking diggable cells.
        if grid.get(gx, gy).is_diggable() {
            dig_zones.cells.insert((gx, gy));
            // Tint the tile to show it's designated.
            for (tile, mut sprite, tile_map) in &mut tile_query {
                if tile_map.0 == active.entity && tile.grid_x == gx && tile.grid_y == gy {
                    sprite.color = Color::srgb(0.6, 0.45, 0.2);
                    break;
                }
            }
        }
    } else if right {
        if dig_zones.cells.remove(&(gx, gy)) {
            // Restore original color.
            let cell = grid.get(gx, gy);
            for (tile, mut sprite, tile_map) in &mut tile_query {
                if tile_map.0 == active.entity && tile.grid_x == gx && tile.grid_y == gy {
                    sprite.color = cell.color();
                    break;
                }
            }
        }
    }
}

// ── Task Labels ───────────────────────────────────────────────────────

#[derive(Component)]
struct NestTaskLabel;

/// Show task letter above each nest ant when viewing any nest.
fn nest_task_labels(
    active: Res<ActiveMap>,
    mut commands: Commands,
    ant_query: Query<(Entity, &NestTask, &MapId, Option<&Children>)>,
    existing_labels: Query<Entity, With<NestTaskLabel>>,
) {
    // Clean up old labels.
    for entity in &existing_labels {
        commands.entity(entity).despawn();
    }

    if !matches!(active.kind, MapKind::Nest { .. }) {
        return;
    }

    for (entity, task, map_id, _children) in &ant_query {
        // Only label ants on the currently viewed nest.
        if map_id.0 != active.entity {
            continue;
        }

        let label = task.label();
        let color = task.color();

        let label_entity = commands
            .spawn((
                Text2d::new(label),
                TextFont {
                    font_size: 9.0,
                    ..default()
                },
                TextColor(color),
                Transform::from_xyz(0.0, 6.0, 0.1),
                NestTaskLabel,
                MapId(map_id.0),
            ))
            .id();

        commands.entity(entity).add_child(label_entity);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

/// Find a passable cell adjacent to the given (diggable) cell.
fn find_adjacent_passable(grid: &NestGrid, x: usize, y: usize) -> Option<(usize, usize)> {
    for &(dx, dy) in &[(-1i32, 0), (1, 0), (0, -1i32), (0, 1)] {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if nx >= 0
            && ny >= 0
            && (nx as usize) < grid.width
            && (ny as usize) < grid.height
            && grid.get(nx as usize, ny as usize).is_passable()
        {
            return Some((nx as usize, ny as usize));
        }
    }
    None
}

/// Find a passable cell belonging to a chamber identified by label index.
fn find_label_cell(grid: &NestGrid, label: usize) -> Option<(usize, usize)> {
    use crate::resources::nest_pheromone::chamber_kind_to_label;

    let cx = NEST_WIDTH / 2;

    if label == LABEL_ENTRANCE {
        for y in 0..grid.height {
            if grid.get(cx, y).is_passable() {
                return Some((cx, y));
            }
        }
        return None;
    }

    for y in 0..grid.height {
        for x in 0..grid.width {
            if let CellType::Chamber(kind) = grid.get(x, y) {
                if chamber_kind_to_label(kind) == label {
                    return Some((x, y));
                }
            }
        }
    }
    None
}
