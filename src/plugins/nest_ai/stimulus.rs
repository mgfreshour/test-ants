use bevy::prelude::*;
use std::collections::HashMap;

use crate::components::ant::{PlayerControlled, StimulusThresholds};
use crate::components::map::{MapId, MapMarker};
use crate::components::nest::{AttendStep, Brood, BroodStage, DigStep, FeedStep, FoodEntity, HaulStep, MoveBroodStep, NestPath, NestTask, Queen, QueenHunger, StackedItem};
use crate::plugins::ant_ai::ColonyFood;
use crate::plugins::nest_navigation::world_to_nest_grid;
use crate::resources::nest::{NestGrid, PlayerDigZones};
use crate::resources::nest_pheromone::NestPheromoneGrid;
use crate::resources::simulation::{SimClock, SimSpeed};
use crate::sim_core::nest_stimuli;

/// Scan radius in grid cells for detecting nearby stimuli.
const SCAN_RADIUS: i32 = 4;
/// How often wandering ants scan for stimuli (seconds).
const SCAN_INTERVAL: f32 = 0.5;

/// Wandering ants scan nearby cells for stimuli and pick up tasks when
/// a stimulus exceeds their personal threshold (set by AntJob).
/// Replaces the old global utility scoring system.
pub(super) fn stimulus_scan(
    mut commands: Commands,
    clock: Res<SimClock>,
    map_query: Query<(&NestGrid, &NestPheromoneGrid, &ColonyFood, Option<&PlayerDigZones>), With<MapMarker>>,
    brood_query: Query<(Entity, &Transform, &Brood, &MapId)>,
    queen_query: Query<(&MapId, &QueenHunger), With<Queen>>,
    food_entity_query: Query<(&Transform, &MapId, Option<&StackedItem>), With<FoodEntity>>,
    mut query: Query<(Entity, &Transform, &MapId, &mut NestTask, &mut StimulusThresholds), Without<PlayerControlled>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    // Pre-compute task counts per map for crowding penalty.
    let mut feed_counts: HashMap<Entity, usize> = HashMap::new();
    let mut move_counts: HashMap<Entity, usize> = HashMap::new();
    let mut haul_counts: HashMap<Entity, usize> = HashMap::new();
    let mut queen_counts: HashMap<Entity, usize> = HashMap::new();
    let mut dig_counts: HashMap<Entity, usize> = HashMap::new();
    for (_, _, m, t, _) in query.iter() {
        match &*t {
            NestTask::FeedLarva { .. } => *feed_counts.entry(m.0).or_insert(0) += 1,
            NestTask::MoveBrood { .. } => *move_counts.entry(m.0).or_insert(0) += 1,
            NestTask::HaulFood { .. } => *haul_counts.entry(m.0).or_insert(0) += 1,
            NestTask::AttendQueen { .. } => *queen_counts.entry(m.0).or_insert(0) += 1,
            NestTask::Dig { .. } => *dig_counts.entry(m.0).or_insert(0) += 1,
            _ => {}
        }
    }

    for (entity, transform, map_id, mut task, thresholds) in &mut query {
        // Only scan wandering ants whose scan timer has elapsed.
        let scan_ready = match &*task {
            NestTask::Wander { scan_timer, .. } => *scan_timer >= SCAN_INTERVAL,
            _ => false,
        };
        if !scan_ready {
            continue;
        }

        // Reset scan timer (keep wander_time accumulating for surface ejection).
        if let NestTask::Wander { scan_timer, .. } = &mut *task {
            *scan_timer = 0.0;
        }

        let Ok((grid, phero_grid, colony_food, dig_zones_opt)) =
            map_query.get(map_id.0) else { continue };

        let pos = transform.translation.truncate();
        let Some(ant_gp) = world_to_nest_grid(pos) else { continue };
        let (ax, ay) = (ant_gp.0 as i32, ant_gp.1 as i32);

        // Collect the best stimulus of each type within scan radius.
        let mut best_strength: [(nest_stimuli::StimulusType, f32); 5] = [
            (nest_stimuli::StimulusType::HungryLarva, 0.0),
            (nest_stimuli::StimulusType::UnrelocatedBrood, 0.0),
            (nest_stimuli::StimulusType::LooseFood, 0.0),
            (nest_stimuli::StimulusType::HungryQueen, 0.0),
            (nest_stimuli::StimulusType::DigFace, 0.0),
        ];

        // ── Hungry larva / unrelocated brood ──
        for (_e, btf, brood, bmap) in &brood_query {
            if bmap.0 != map_id.0 { continue; }
            let bpos = btf.translation.truncate();
            let Some(bgp) = world_to_nest_grid(bpos) else { continue };
            let dx = bgp.0 as i32 - ax;
            let dy = bgp.1 as i32 - ay;
            if dx.abs() > SCAN_RADIUS || dy.abs() > SCAN_RADIUS { continue; }
            let dist = ((dx * dx + dy * dy) as f32).sqrt();

            if brood.stage == BroodStage::Larva && !brood.fed && colony_food.stored > 0.5 {
                let phero = phero_grid.get(bgp.0, bgp.1).brood_need;
                let s = nest_stimuli::larva_stimulus_strength(dist, phero);
                if s > best_strength[0].1 { best_strength[0].1 = s; }
            }
            if !brood.relocated {
                let s = nest_stimuli::brood_stimulus_strength(dist);
                if s > best_strength[1].1 { best_strength[1].1 = s; }
            }
        }

        // ── Loose food at entrance ──
        for (ftf, fmap, stacked) in &food_entity_query {
            if fmap.0 != map_id.0 || stacked.is_some() { continue; }
            let fpos = ftf.translation.truncate();
            let Some(fgp) = world_to_nest_grid(fpos) else { continue };
            let dx = fgp.0 as i32 - ax;
            let dy = fgp.1 as i32 - ay;
            if dx.abs() > SCAN_RADIUS || dy.abs() > SCAN_RADIUS { continue; }
            let dist = ((dx * dx + dy * dy) as f32).sqrt();
            let s = nest_stimuli::food_stimulus_strength(dist);
            if s > best_strength[2].1 { best_strength[2].1 = s; }
        }

        // ── Hungry queen (pheromone-based, not proximity to queen entity) ──
        if let Some((_, hunger)) = queen_query.iter().find(|(qm, _)| qm.0 == map_id.0) {
            let queen_hunger_val = 1.0 - hunger.satiation.clamp(0.0, 1.0);
            let queen_signal = phero_grid.get(ant_gp.0, ant_gp.1).queen_signal;
            if queen_hunger_val > 0.1 && colony_food.stored > 0.5 {
                let s = nest_stimuli::queen_stimulus_strength(queen_hunger_val, queen_signal);
                best_strength[3].1 = s;
            }
        }

        // ── Dig faces nearby ──
        let has_dig_zones = dig_zones_opt.map_or(false, |dz| !dz.cells.is_empty());
        for dy in -SCAN_RADIUS..=SCAN_RADIUS {
            for dx in -SCAN_RADIUS..=SCAN_RADIUS {
                let nx = ax + dx;
                let ny = ay + dy;
                if nx < 0 || ny < 0 || nx as usize >= grid.width || ny as usize >= grid.height {
                    continue;
                }
                let (ux, uy) = (nx as usize, ny as usize);
                let cell = grid.get(ux, uy);
                if !cell.is_diggable() { continue; }
                // Must be adjacent to a passable cell (i.e., a dig face).
                let is_face = [(-1i32, 0), (1, 0), (0, -1i32), (0, 1)].iter().any(|&(fdx, fdy)| {
                    let px = nx + fdx;
                    let py = ny + fdy;
                    px >= 0 && py >= 0
                        && (px as usize) < grid.width
                        && (py as usize) < grid.height
                        && grid.get(px as usize, py as usize).is_passable()
                });
                if !is_face { continue; }

                let construction = phero_grid.get(ux, uy).construction;
                let player_bonus = if has_dig_zones && dig_zones_opt.unwrap().cells.contains(&(ux, uy)) {
                    0.5
                } else {
                    0.0
                };
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                let s = nest_stimuli::dig_stimulus_strength(dist, construction + player_bonus);
                if s > best_strength[4].1 { best_strength[4].1 = s; }
            }
        }

        // Pick the stimulus that most exceeds its threshold.
        let workers_per_type = [
            *feed_counts.get(&map_id.0).unwrap_or(&0),
            *move_counts.get(&map_id.0).unwrap_or(&0),
            *haul_counts.get(&map_id.0).unwrap_or(&0),
            *queen_counts.get(&map_id.0).unwrap_or(&0),
            *dig_counts.get(&map_id.0).unwrap_or(&0),
        ];

        let mut best_idx: Option<usize> = None;
        let mut best_margin: f32 = 0.0;

        for (i, &(stype, strength)) in best_strength.iter().enumerate() {
            let threshold = thresholds_get(&thresholds, stype);
            if nest_stimuli::should_respond(strength, threshold, workers_per_type[i]) {
                let margin = strength - threshold;
                if margin > best_margin {
                    best_margin = margin;
                    best_idx = Some(i);
                }
            }
        }

        if let Some(idx) = best_idx {
            // Remove any leftover NestPath from wandering so the new task
            // starts fresh with pathfinding to its own destination.
            commands.entity(entity).remove::<NestPath>();
            *task = match best_strength[idx].0 {
                nest_stimuli::StimulusType::HungryLarva => NestTask::FeedLarva {
                    step: FeedStep::GoToStorage,
                    target_larva: None,
                },
                nest_stimuli::StimulusType::UnrelocatedBrood => NestTask::MoveBrood {
                    step: MoveBroodStep::GoToQueen,
                    target_brood: None,
                },
                nest_stimuli::StimulusType::LooseFood => NestTask::HaulFood {
                    step: HaulStep::GoToEntrance,
                },
                nest_stimuli::StimulusType::HungryQueen => NestTask::AttendQueen {
                    step: AttendStep::GoToStorage,
                },
                nest_stimuli::StimulusType::DigFace => NestTask::Dig {
                    step: DigStep::GoToFace,
                    target_cell: None,
                    dig_timer: 0.0,
                },
            };
        }
    }
}

/// Helper to read threshold from StimulusThresholds component by StimulusType.
fn thresholds_get(t: &StimulusThresholds, stype: nest_stimuli::StimulusType) -> f32 {
    match stype {
        nest_stimuli::StimulusType::HungryLarva => t.feed_larva,
        nest_stimuli::StimulusType::UnrelocatedBrood => t.move_brood,
        nest_stimuli::StimulusType::LooseFood => t.haul_food,
        nest_stimuli::StimulusType::HungryQueen => t.attend_queen,
        nest_stimuli::StimulusType::DigFace => t.dig,
    }
}
