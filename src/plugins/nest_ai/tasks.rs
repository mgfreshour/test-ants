use bevy::prelude::*;
use rand::Rng;

use crate::components::map::{MapId, MapMarker};
use crate::components::nest::{AttendStep, Brood, BroodStage, CarriedBy, CellType, ChamberKind, DigStep, FeedStep, FoodEntity, HaulStep, MoveBroodStep, NestPath, NestTask, Queen, QueenHunger, StackedItem};
use crate::plugins::ant_ai::ColonyFood;
use crate::plugins::nest_navigation::{nest_grid_to_world, world_to_nest_grid};
use crate::resources::nest::{NestGrid, PlayerDigZones, TileStackRegistry, stack_position_offset};
use crate::resources::nest_pathfinding::NestPathCache;
use crate::resources::nest_pheromone::{NestPheromoneGrid, LABEL_BROOD, LABEL_ENTRANCE, LABEL_FOOD_STORAGE, LABEL_QUEEN};
use crate::resources::simulation::{SimClock, SimSpeed};
use crate::sim_core::{regressions, nest_transitions};

use super::{ant_at_destination, find_label_cell, find_adjacent_passable, BroodFed, BroodRelocated, ExcavatedCell, ExpandZone};

/// Advance FeedLarva task sub-steps.
pub(super) fn advance_feed_task(
    clock: Res<SimClock>,
    mut map_query: Query<(&NestGrid, &mut NestPathCache, &mut ColonyFood, &mut TileStackRegistry), With<MapMarker>>,
    mut commands: Commands,
    brood_query: Query<(Entity, &Transform, &Brood)>,
    food_entity_query: Query<(Entity, &Transform, &FoodEntity, Option<&StackedItem>)>,
    carried_food_query: Query<(Entity, &CarriedBy), With<FoodEntity>>,
    mut ant_query: Query<(Entity, &Transform, &MapId, &mut NestTask, Option<&NestPath>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    // Pre-index unfed larvae: Vec<(Entity, Vec2)>.
    let unfed_larvae: Vec<(Entity, Vec2)> = brood_query
        .iter()
        .filter(|(_, _, b)| b.stage == BroodStage::Larva && !b.fed)
        .map(|(e, tf, _)| (e, tf.translation.truncate()))
        .collect();

    // Pre-index food with stacking info for storage lookups.
    let all_food: Vec<(Entity, Vec2, Option<((usize, usize), u8)>)> = food_entity_query
        .iter()
        .map(|(e, tf, _, s)| (e, tf.translation.truncate(), s.map(|si| (si.grid_pos, si.stack_index))))
        .collect();

    // Pre-index carried food by carrier entity.
    let carried_by_ant: std::collections::HashMap<Entity, Entity> = carried_food_query
        .iter()
        .map(|(food_e, cb)| (cb.0, food_e))
        .collect();

    for (entity, transform, map_id, mut task, path) in &mut ant_query {
        let NestTask::FeedLarva { ref mut step, ref mut target_larva } = *task else { continue };

        let Ok((grid, mut path_cache, mut colony_food, mut stack_registry)) =
            map_query.get_mut(map_id.0) else { continue };

        let pos = transform.translation.truncate();
        let grid_pos = match world_to_nest_grid(pos) {
            Some(gp) => gp,
            None => continue,
        };
        let at_destination = ant_at_destination(path);

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
                                *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                                continue;
                            }
                        } else {
                            *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                            continue;
                        }
                    }
                }
            }
            FeedStep::PickUpFood => {
                if at_destination {
                    // Find nearest stacked food in storage from pre-indexed Vec.
                    let storage_food = all_food
                        .iter()
                        .filter(|(_, _, s)| {
                            s.map_or(false, |(gp, _)| {
                                grid.get(gp.0, gp.1) == CellType::Chamber(ChamberKind::FoodStorage)
                            })
                        })
                        .min_by_key(|(_, fpos, _)| {
                            (pos.distance(*fpos) * 100.0) as i32
                        });

                    if let Some(&(food_e, _, stacked_info)) = storage_food {
                        if let Some((gp, _)) = stacked_info {
                            stack_registry.remove(gp, food_e);
                        }
                        commands.entity(food_e).remove::<StackedItem>();
                        commands.entity(food_e).insert(CarriedBy(entity));
                        commands.entity(entity).remove::<NestPath>();
                        *step = FeedStep::GoToBrood;
                    } else {
                        // No food available, go idle.
                        *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
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
                                *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                                continue;
                            }
                        } else {
                            *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                            continue;
                        }
                    }
                }
            }
            FeedStep::FindLarva => {
                if at_destination {
                    // Find nearest unfed larva from pre-indexed Vec.
                    let best = unfed_larvae
                        .iter()
                        .map(|&(e, lpos)| (e, pos.distance(lpos)))
                        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

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

                // Despawn carried food entity from pre-indexed map.
                if let Some(&food_e) = carried_by_ant.get(&entity) {
                    commands.entity(food_e).despawn();
                    colony_food.stored -= 1.0;
                }

                *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                continue;
            }
        }
    }
}

/// Advance MoveBrood task sub-steps.
pub(super) fn advance_move_brood_task(
    clock: Res<SimClock>,
    mut map_query: Query<(&NestGrid, &mut NestPathCache), With<MapMarker>>,
    mut commands: Commands,
    brood_query: Query<(Entity, &Transform, &Brood)>,
    mut ant_query: Query<(Entity, &Transform, &MapId, &mut NestTask, Option<&NestPath>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    // Pre-index unrelocated brood: Vec<(Entity, Vec2)>.
    let unrelocated_brood: Vec<(Entity, Vec2)> = brood_query
        .iter()
        .filter(|(_, _, b)| !b.relocated)
        .map(|(e, tf, _)| (e, tf.translation.truncate()))
        .collect();

    for (entity, transform, map_id, mut task, path) in &mut ant_query {
        let NestTask::MoveBrood { ref mut step, ref mut target_brood } = *task else { continue };

        let Ok((grid, mut path_cache)) =
            map_query.get_mut(map_id.0) else { continue };

        let pos = transform.translation.truncate();
        let grid_pos = match world_to_nest_grid(pos) {
            Some(gp) => gp,
            None => continue,
        };
        let at_destination = ant_at_destination(path);

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
                                *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                                continue;
                            }
                        } else {
                            *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                            continue;
                        }
                    }
                }
            }
            MoveBroodStep::PickUpBrood => {
                if at_destination {
                    // Find nearest unrelocated brood from pre-indexed Vec.
                    let best = unrelocated_brood
                        .iter()
                        .map(|&(e, bpos)| (e, pos.distance(bpos)))
                        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

                    if let Some((brood_entity, _)) = best {
                        *target_brood = Some(brood_entity);
                        // Attach brood to ant so it follows.
                        commands.entity(brood_entity).insert(CarriedBy(entity));
                        commands.entity(entity).remove::<NestPath>();
                        *step = MoveBroodStep::GoToBrood;
                    } else {
                        *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
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
                                *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                                continue;
                            }
                        } else {
                            *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
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
                    *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                    continue;
                }
            }
        }
    }
}

/// Advance HaulFood task sub-steps.
pub(super) fn advance_haul_task(
    clock: Res<SimClock>,
    mut map_query: Query<(&NestGrid, &mut NestPathCache, &mut TileStackRegistry), With<MapMarker>>,
    mut commands: Commands,
    food_entity_query: Query<(Entity, &Transform, &FoodEntity, Option<&StackedItem>)>,
    carried_food_query: Query<(Entity, &CarriedBy), With<FoodEntity>>,
    mut ant_query: Query<(Entity, &Transform, &MapId, &mut NestTask, Option<&NestPath>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    // Pre-index unstacked food with grid positions for entrance check.
    let unstacked_food: Vec<(Entity, Vec2, Option<(usize, usize)>)> = food_entity_query
        .iter()
        .filter(|(_, _, _, s)| s.is_none())
        .map(|(e, tf, _, _)| {
            let fpos = tf.translation.truncate();
            (e, fpos, world_to_nest_grid(fpos))
        })
        .collect();

    // Pre-index carried food by carrier entity.
    let carried_by_ant: std::collections::HashMap<Entity, Entity> = carried_food_query
        .iter()
        .map(|(food_e, cb)| (cb.0, food_e))
        .collect();

    for (entity, transform, map_id, mut task, path) in &mut ant_query {
        let NestTask::HaulFood { ref mut step } = *task else { continue };

        let Ok((grid, mut path_cache, mut stack_registry)) =
            map_query.get_mut(map_id.0) else { continue };

        let pos = transform.translation.truncate();
        let grid_pos = match world_to_nest_grid(pos) {
            Some(gp) => gp,
            None => continue,
        };
        let at_destination = ant_at_destination(path);

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
                                *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                                continue;
                            }
                        } else {
                            *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                            continue;
                        }
                    }
                }
            }
            HaulStep::PickUpFood => {
                if at_destination {
                    // Find nearest non-stacked food entity at entrance from pre-indexed Vec.
                    let cx = grid.width / 2;
                    let best = unstacked_food
                        .iter()
                        .filter(|(_, _, gp)| {
                            gp.map_or(false, |(gx, gy)| gx == cx && gy <= 6)
                        })
                        .map(|&(e, fpos, _)| (e, pos.distance(fpos)))
                        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

                    if let Some((food_entity, _)) = best {
                        commands.entity(food_entity).insert(CarriedBy(entity));
                        commands.entity(entity).remove::<NestPath>();
                        *step = HaulStep::GoToStorage;
                    } else {
                        *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
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
                                *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                                continue;
                            }
                        } else {
                            *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                            continue;
                        }
                    }
                }
            }
            HaulStep::DropFood => {
                if at_destination {
                    // Find carried food entity from pre-indexed map.
                    let carried_food_e = carried_by_ant.get(&entity).copied();

                    if let Some(food_e) = carried_food_e {
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
                    *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                    continue;
                }
            }
        }
    }
}

/// Advance AttendQueen task sub-steps.
pub(super) fn advance_attend_queen_task(
    clock: Res<SimClock>,
    mut map_query: Query<(&NestGrid, &mut NestPathCache, &mut ColonyFood, &mut TileStackRegistry), With<MapMarker>>,
    mut commands: Commands,
    food_entity_query: Query<(Entity, &Transform, &FoodEntity, Option<&StackedItem>)>,
    carried_food_query: Query<(Entity, &CarriedBy), With<FoodEntity>>,
    mut queen_hunger_query: Query<(&mut QueenHunger, &MapId), With<Queen>>,
    mut ant_query: Query<(Entity, &Transform, &MapId, &mut NestTask, Option<&NestPath>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    // Pre-index food with stacking info for storage lookups.
    let all_food: Vec<(Entity, Vec2, Option<((usize, usize), u8)>)> = food_entity_query
        .iter()
        .map(|(e, tf, _, s)| (e, tf.translation.truncate(), s.map(|si| (si.grid_pos, si.stack_index))))
        .collect();

    // Pre-index carried food by carrier entity.
    let carried_by_ant: std::collections::HashMap<Entity, Entity> = carried_food_query
        .iter()
        .map(|(food_e, cb)| (cb.0, food_e))
        .collect();

    for (entity, transform, map_id, mut task, path) in &mut ant_query {
        let NestTask::AttendQueen { ref mut step } = *task else { continue };

        let Ok((grid, mut path_cache, mut colony_food, mut stack_registry)) =
            map_query.get_mut(map_id.0) else { continue };

        let pos = transform.translation.truncate();
        let grid_pos = match world_to_nest_grid(pos) {
            Some(gp) => gp,
            None => continue,
        };
        let at_destination = ant_at_destination(path);

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
                                *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                                continue;
                            }
                        } else {
                            *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                            continue;
                        }
                    }
                }
            }
            AttendStep::PickUpFood => {
                if at_destination {
                    // Find nearest stacked food in storage from pre-indexed Vec.
                    let storage_food = all_food
                        .iter()
                        .filter(|(_, _, s)| {
                            s.map_or(false, |(gp, _)| {
                                grid.get(gp.0, gp.1) == CellType::Chamber(ChamberKind::FoodStorage)
                            })
                        })
                        .min_by_key(|(_, fpos, _)| {
                            (pos.distance(*fpos) * 100.0) as i32
                        });

                    if let Some(&(food_e, _, stacked_info)) = storage_food {
                        if let Some((gp, _)) = stacked_info {
                            stack_registry.remove(gp, food_e);
                        }
                        commands.entity(food_e).remove::<StackedItem>();
                        commands.entity(food_e).insert(CarriedBy(entity));
                        commands.entity(entity).remove::<NestPath>();
                        *step = AttendStep::GoToQueen;
                    } else {
                        // No food available, go idle.
                        *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
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
                                *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                                continue;
                            }
                        } else {
                            *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                            continue;
                        }
                    }
                }
            }
            AttendStep::FeedQueen => {
                if at_destination {
                    // Find our carried food from pre-indexed map.
                    let carried_food_e = carried_by_ant.get(&entity).copied();

                    if let Some(food_e) = carried_food_e {
                        // Transfer food to queen on the same map as this ant.
                        for (mut hunger, queen_map) in &mut queen_hunger_query {
                            if queen_map.0 == map_id.0 {
                                hunger.satiation = (hunger.satiation + 0.25).min(1.0);
                                break;
                            }
                        }
                        commands.entity(food_e).despawn();
                        colony_food.stored -= 1.0;
                    }

                    *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                    continue;
                }
            }
        }
    }
}

/// Advance Dig task sub-steps.
pub(super) fn advance_dig_task(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut map_query: Query<(&NestGrid, &NestPheromoneGrid, &mut NestPathCache, Option<&PlayerDigZones>), With<MapMarker>>,
    mut commands: Commands,
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
        let NestTask::Dig { ref mut step, ref mut target_cell, ref mut dig_timer } = *task else { continue };

        let Ok((grid, phero_grid, mut path_cache, dig_zones_opt)) =
            map_query.get_mut(map_id.0) else { continue };

        let pos = transform.translation.truncate();
        let grid_pos = match world_to_nest_grid(pos) {
            Some(gp) => gp,
            None => continue,
        };
        let at_destination = ant_at_destination(path);
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
                            *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
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
                                    *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                                    continue;
                                }
                            } else {
                                *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                                continue;
                            }
                        } else {
                            *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
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
                        *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                        continue;
                    }
                }
            }
            // PickUpSoil, GoToMidden, DropSoil unused — soil vanishes on excavation.
            _ => {
                *task = NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 };
                continue;
            }
        }
    }
}

/// Advance Wander task: increment timers and pathfind to random passable cells.
/// Wandering ants are always moving — when their path completes they pick a new
/// random target within ~8 grid cells.
pub(super) fn advance_wander_task(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut map_query: Query<(&NestGrid, &mut NestPathCache), With<MapMarker>>,
    mut commands: Commands,
    mut query: Query<(Entity, &Transform, &MapId, &mut NestTask, Option<&NestPath>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    let mut rng = rand::thread_rng();

    for (entity, transform, map_id, mut task, path) in &mut query {
        let NestTask::Wander { ref mut scan_timer, ref mut wander_time } = *task else { continue };

        *scan_timer += dt;
        *wander_time += dt;

        // If we have no path or path is complete, pick a new random target.
        let needs_path = path.map_or(true, |p| p.is_complete());
        if !needs_path {
            continue;
        }

        // Remove completed path.
        if path.is_some() {
            commands.entity(entity).remove::<NestPath>();
        }

        let Ok((grid, mut path_cache)) = map_query.get_mut(map_id.0) else { continue };

        let pos = transform.translation.truncate();
        let Some(ant_gp) = world_to_nest_grid(pos) else { continue };

        // Pick a random passable cell within ~8 grid cells.
        let wander_range = 8i32;
        let mut attempts = 0;
        while attempts < 5 {
            attempts += 1;
            let dx = rng.gen_range(-wander_range..=wander_range);
            let dy = rng.gen_range(-wander_range..=wander_range);
            let tx = ant_gp.0 as i32 + dx;
            let ty = ant_gp.1 as i32 + dy;
            if tx < 0 || ty < 0 || tx as usize >= grid.width || ty as usize >= grid.height {
                continue;
            }
            let target = (tx as usize, ty as usize);
            if !grid.get(target.0, target.1).is_passable() {
                continue;
            }
            if target == ant_gp {
                continue;
            }
            if let Some(waypoints) = path_cache.find_path(&grid, ant_gp, target) {
                commands.entity(entity).insert(NestPath::new(waypoints));
                break;
            }
        }
    }
}
