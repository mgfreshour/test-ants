use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{Ant, AntJob, AntState, ColonyMember, Health, Movement, PlayerControlled, PortalCooldown, PositionHistory, StimulusThresholds, Underground};
use crate::components::map::{MapId, MapMarker, MapPortal, PORTAL_RANGE};
use crate::components::nest::{Brood, CellType, ChamberKind, NestPath, NestTask, StackedItem};
use crate::plugins::nest_navigation::nest_grid_to_world;
use crate::resources::active_map::MapRegistry;
use crate::resources::colony::BehaviorSliders;
use crate::resources::nest::{NestGrid, TileStackRegistry, stack_position_offset};
use crate::resources::nest_pathfinding::NestPathCache;
use crate::resources::nest_pheromone::NestPheromoneGrid;
use crate::resources::simulation::{SimClock, SimSpeed};
use crate::sim_core::regressions;

use super::{BroodFed, BroodRelocated, ExpandZoneDeferred};

/// Tick down portal cooldown timers and remove expired ones.
pub(super) fn tick_portal_cooldowns(
    mut commands: Commands,
    time: Res<Time>,
    clock: Res<SimClock>,
    mut query: Query<(Entity, &mut PortalCooldown)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }
    let dt = time.delta_secs() * clock.speed.multiplier();
    for (entity, mut cd) in &mut query {
        cd.remaining -= dt;
        if cd.remaining <= 0.0 {
            commands.entity(entity).remove::<PortalCooldown>();
        }
    }
}

/// Apply flood damage to nest ants when water level is high.
pub(super) fn apply_flood_damage(
    env: Res<crate::plugins::environment::EnvironmentState>,
    mut query: Query<&mut Health, With<Underground>>,
) {
    if env.flood_level > 0.1 {
        let damage = env.flood_level * 10.0; // Scale damage by flood level
        for mut health in &mut query {
            health.current -= damage * 0.016; // Per-frame damage
        }
    }
}

/// Apply the BroodFed marker component to actually set brood.fed = true.
pub(super) fn apply_brood_fed(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Brood), With<BroodFed>>,
) {
    for (entity, mut brood) in &mut query {
        brood.fed = true;
        commands.entity(entity).remove::<BroodFed>();
    }
}

/// Apply the BroodRelocated marker: set relocated = true and move brood to brood chamber.
pub(super) fn apply_brood_relocated(
    mut commands: Commands,
    mut map_query: Query<(&NestGrid, &mut TileStackRegistry), With<MapMarker>>,
    mut query: Query<(Entity, &mut Brood, &mut Transform, &MapId), With<BroodRelocated>>,
) {
    for (entity, mut brood, mut transform, map_id) in &mut query {
        brood.relocated = true;

        let Ok((grid, mut stack_registry)) = map_query.get_mut(map_id.0) else {
            commands.entity(entity).remove::<BroodRelocated>();
            continue;
        };

        let tile_pos = stack_registry
            .find_available_tile(&grid, ChamberKind::Brood)
            .or_else(|| {
                grid.find_expansion_candidate(ChamberKind::Brood).map(|exp| {
                    (exp.x, exp.y)
                })
            });

        if let Some(tile_pos) = tile_pos {
            if grid.get(tile_pos.0, tile_pos.1) == CellType::Tunnel {
                commands.spawn(ExpandZoneDeferred {
                    x: tile_pos.0,
                    y: tile_pos.1,
                    chamber: ChamberKind::Brood,
                    map: map_id.0,
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

/// Process deferred zone expansions from brood relocation.
pub(super) fn apply_deferred_zone_expansions(
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

/// Generic portal transition: any ant within PORTAL_RANGE of a portal mouth on
/// its current map (and whose colony passes the restriction) transitions to the
/// target map at the target position.
pub(super) fn portal_transition(
    clock: Res<SimClock>,
    sliders_query: Query<&BehaviorSliders, With<MapMarker>>,
    registry: Res<MapRegistry>,
    portal_query: Query<&MapPortal>,
    mut ant_query: Query<
        (Entity, &mut Transform, &mut Ant, &ColonyMember, &mut MapId, &mut Visibility, Option<&NestTask>, &AntJob, Option<&PortalCooldown>),
        Without<PlayerControlled>,
    >,
    mut commands: Commands,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    // Count total ants per colony and underground job-specific ants per nest map.
    let mut total_per_colony: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    let mut underground_jobs_per_nest: std::collections::HashMap<Entity, (usize, usize)> = std::collections::HashMap::new(); // (nurse_count, digger_count)
    for (_, _, _, colony, map_id, _, nest_task, job, _) in &ant_query {
        *total_per_colony.entry(colony.colony_id).or_insert(0) += 1;
        if nest_task.is_some() {
            let (nurse_count, digger_count) = underground_jobs_per_nest.entry(map_id.0).or_insert((0, 0));
            match job {
                AntJob::Nurse => *nurse_count += 1,
                AntJob::Digger => *digger_count += 1,
                _ => {}
            }
        }
    }

    let mut rng = rand::thread_rng();

    for (entity, mut transform, mut ant, colony, mut map_id, mut vis, _, job, cooldown) in &mut ant_query {
        // Skip ants with active portal cooldown.
        if cooldown.is_some() {
            continue;
        }
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

            let target_is_nest = portal.target_map != registry.surface;
            if target_is_nest {
                let is_following = ant.state == AntState::Following;

                // Job-specific capacity counting: only Nurse and Digger count underground.
                let (nurse_count, digger_count) = underground_jobs_per_nest.get(&portal.target_map).copied().unwrap_or((0, 0));
                let current_underground = nurse_count + digger_count;

                let total_ants = *total_per_colony.get(&colony.colony_id).unwrap_or(&0);
                let desired_underground = sliders_query.get(portal.target_map).ok()
                    .map(|s| ((s.nurse + s.dig) * total_ants as f32).ceil() as usize)
                    .unwrap_or(0);

                if !is_following && !regressions::should_enter_nest(
                    current_underground,
                    desired_underground,
                    *job,
                    rng.gen::<f32>(),
                    0.02,
                ) {
                    break;
                }

                ant.state = if is_following { AntState::Following } else { AntState::Idle };
                map_id.0 = portal.target_map;
                transform.translation.x = portal.target_position.x;
                transform.translation.y = portal.target_position.y;
                *vis = Visibility::Hidden;
                commands.entity(entity).insert((
                    NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 },
                    StimulusThresholds::from_job(*job),
                    PortalCooldown::new(),
                ));
            } else {
                if ant.state == AntState::Following {
                    ant.state = AntState::Following;
                    map_id.0 = portal.target_map;
                    transform.translation.x = portal.target_position.x + rng.gen_range(-15.0..15.0f32);
                    transform.translation.y = portal.target_position.y + rng.gen_range(-15.0..15.0f32);
                    *vis = Visibility::Inherited;
                    commands.entity(entity).remove::<NestTask>();
                    commands.entity(entity).remove::<NestPath>();
                    commands.entity(entity).insert(PortalCooldown::new());
                }
            }
            break;
        }
    }
}

/// Nest ants that have been idle too long exit through a portal back to the surface.
/// Job-based exit logic: Forager and Defender ants exit after 5s idle; Nurse and Digger stay underground.
pub(super) fn nest_to_surface_transition(
    clock: Res<SimClock>,
    time: Res<Time>,
    config: Res<crate::resources::simulation::SimConfig>,
    registry: Res<MapRegistry>,
    portal_query: Query<&MapPortal>,
    mut commands: Commands,
    mut query: Query<
        (Entity, &mut Transform, &mut Ant, &mut NestTask, &mut MapId, &mut Visibility, &AntJob, &mut Movement, &mut PositionHistory),
    >,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    let mut rng = rand::thread_rng();

    for (entity, mut transform, mut ant, mut task, mut map_id, mut vis, job, mut movement, mut history) in &mut query {
        // Only process ants currently in a nest.
        if map_id.0 == registry.surface {
            continue;
        }

        if let NestTask::Wander { ref mut wander_time, .. } = *task {
            *wander_time += dt;

            // Job-based exit logic: Forager and Defender ants exit after 5s idle; underground jobs stay.
            let should_exit = match job {
                AntJob::Forager | AntJob::Defender => *wander_time > 5.0,   // Surface jobs exit after 5s
                AntJob::Nurse | AntJob::Digger => false,                    // Underground jobs never exit
                AntJob::Unassigned => *wander_time > 10.0,                  // Unassigned gracefully ejected
            };

            if should_exit {
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

                // Reset movement for surface: randomize direction and use surface speed.
                let angle = rng.gen::<f32>() * std::f32::consts::TAU;
                movement.direction = Vec2::new(angle.cos(), angle.sin());
                movement.speed = config.ant_speed_worker;
                history.clear();

                commands
                    .entity(entity)
                    .remove::<NestTask>()
                    .remove::<NestPath>()
                    .insert(PortalCooldown::new());
            }
        }
    }
}
