// Nest underground AI systems: task execution, pathfinding, pheromones, excavation
// This module coordinates underground ant behavior, including utility scoring,
// task transitions, job assignment, and excavation logic.

mod transitions;
mod stimulus;
mod tasks;
mod excavation;

use bevy::prelude::*;
use rand::Rng;
use std::collections::HashMap;

use crate::components::ant::{Ant, AntJob, AntState, ColonyMember, Health, Movement, PositionHistory, StimulusThresholds, SteeringTarget, SteeringWeights};
use crate::components::map::{MapId, MapKind, MapMarker};
use crate::components::nest::{CellType, ChamberKind, NestPath, NestTask};
use crate::plugins::ant_ai::ColonyFood;
use crate::plugins::nest_navigation::{nest_grid_to_world, nest_grid_collision};
use crate::resources::colony::BehaviorSliders;
use crate::resources::nest::{NestGrid, NEST_WIDTH};
use crate::resources::nest_pheromone::LABEL_ENTRANCE;
use crate::sim_core::job_assignment::{self, JobAssignmentInput, JobCounts, JobRatios};
use crate::sim_core::nest_transitions;

pub struct NestAiPlugin;

#[derive(Resource)]
struct JobAssignmentTimer {
    timer: Timer,
}

impl Default for JobAssignmentTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(3.0, TimerMode::Repeating),
        }
    }
}

impl Plugin for NestAiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JobAssignmentTimer>()
            .add_systems(Startup, spawn_initial_nest_ants)
            .add_systems(
                Update,
                (
                    (
                        transitions::apply_flood_damage,
                        transitions::apply_brood_fed,
                        transitions::apply_brood_relocated,
                        transitions::apply_deferred_zone_expansions,
                        excavation::cleanup_orphaned_carried_items,
                        excavation::update_carried_item_positions,
                        excavation::apply_zone_expansions,
                        excavation::apply_excavated_cells,
                        transitions::portal_transition,
                        transitions::nest_to_surface_transition,
                        job_assignment_system,
                        stimulus::stimulus_scan,
                    ).chain(),
                    (
                        tasks::advance_feed_task,
                        tasks::advance_move_brood_task,
                        tasks::advance_haul_task,
                        tasks::advance_attend_queen_task,
                        tasks::advance_dig_task,
                        tasks::advance_wander_task,
                        excavation::construction_pheromone_deposit,
                        excavation::nest_separation_steering,
                        nest_grid_collision,
                        excavation::player_dig_zone_input,
                        excavation::nest_task_labels,
                    ).chain(),
                ).chain(),
            );
    }
}

// ── Marker Components (shared across submodules) ─────────────────────

/// Temporary marker to feed brood (since we can't mutate Brood in the same query).
#[derive(Component)]
struct BroodFed;

/// Temporary marker: brood has been relocated to the brood chamber.
#[derive(Component)]
struct BroodRelocated;

/// Deferred zone expansion marker (spawned as entity, not component on ant).
#[derive(Component)]
struct ExpandZoneDeferred {
    x: usize,
    y: usize,
    chamber: ChamberKind,
    map: Entity,
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

#[derive(Component)]
struct NestTaskLabel;

// ── Constants ────────────────────────────────────────────────────────

/// Number of initial underground ants spawned at startup.
const INITIAL_NEST_ANTS: usize = 20;
/// Initial food units placed in the nest to bootstrap queen feeding.
const INITIAL_NEST_FOOD: f32 = 10.0;

// ── Shared Helpers ──────────────────────────────────────────────────

/// Shared preamble: compute destination state from path component.
#[inline]
pub(super) fn ant_at_destination(path: Option<&NestPath>) -> bool {
    let path_done = path.map_or(false, |p| p.is_complete());
    let has_no_path = path.is_none();
    nest_transitions::at_destination(path_done, has_no_path)
}

/// Find a passable cell adjacent to the given (diggable) cell.
pub(super) fn find_adjacent_passable(grid: &NestGrid, x: usize, y: usize) -> Option<(usize, usize)> {
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
pub(super) fn find_label_cell(grid: &NestGrid, label: usize) -> Option<(usize, usize)> {
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

// ── Spawn initial underground ants ──────────────────────────────────

fn spawn_initial_nest_ants(
    mut commands: Commands,
    mut food_query: Query<&mut ColonyFood, With<MapMarker>>,
    map_query: Query<(Entity, &NestGrid, &MapKind), With<MapMarker>>,
) {
    let mut rng = rand::thread_rng();

    for (map_entity, grid, kind) in &map_query {
        let MapKind::Nest { colony_id } = kind else { continue };

        // Seed colony with initial food so nurses can feed queen immediately.
        if let Ok(mut food) = food_query.get_mut(map_entity) {
            food.stored += INITIAL_NEST_FOOD;
        }

        let passable: Vec<(usize, usize)> = (0..grid.height)
            .flat_map(|y| (0..grid.width).map(move |x| (x, y)))
            .filter(|&(x, y)| grid.get(x, y).is_passable())
            .collect();

        if passable.is_empty() {
            continue;
        }

        let color = if *colony_id == 0 {
            Color::srgb(0.35, 0.25, 0.15)
        } else {
            Color::srgb(0.55, 0.18, 0.12)
        };

        for i in 0..INITIAL_NEST_ANTS {
            let &(gx, gy) = &passable[rng.gen_range(0..passable.len())];
            let pos = nest_grid_to_world(gx, gy);
            let jitter_x = rng.gen_range(-3.0..3.0);
            let jitter_y = rng.gen_range(-3.0..3.0);

            let age = rng.gen_range(0.0..300.0);
            // Mix of jobs: Nurses (0-7), Diggers (8-13), Foragers (14-19).
            // Foragers will naturally exit to surface via nest_to_surface_transition.
            let job = match i {
                0..=7 => AntJob::Nurse,
                8..=13 => AntJob::Digger,
                _ => AntJob::Forager,
            };

            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(5.0)),
                    ..default()
                },
                Transform::from_xyz(pos.x + jitter_x, pos.y + jitter_y, 2.5),
                Visibility::Hidden,
                Ant {
                    caste: crate::components::ant::Caste::Worker,
                    state: AntState::Idle,
                    age,
                    hunger: 0.0,
                },
                job,
                StimulusThresholds::from_job(job),
                Health::worker(),
                ColonyMember { colony_id: *colony_id },
                MapId(map_entity),
                NestTask::Wander { scan_timer: 0.0, wander_time: 0.0 },
                Movement { speed: 20.0, direction: Vec2::ZERO },
                PositionHistory::default(),
                SteeringTarget::default(),
                SteeringWeights::default(),
            ));
        }
    }
}

/// Assign and rebalance AntJob components to enforce BehaviorSliders ratios.
/// Runs every ~3 seconds and uses age-based affinity + hysteresis to prevent oscillation.
fn job_assignment_system(
    time: Res<Time>,
    mut timer: ResMut<JobAssignmentTimer>,
    sliders_query: Query<&BehaviorSliders, With<MapMarker>>,
    mut ant_query: Query<(&mut AntJob, &Ant, &ColonyMember)>,
) {
    timer.timer.tick(time.delta());
    if !timer.timer.just_finished() {
        return;
    }

    // Collect job data by colony
    let mut ants_by_colony: HashMap<u32, Vec<(AntJob, f32)>> = HashMap::new();
    for (job, ant, colony) in &ant_query {
        ants_by_colony
            .entry(colony.colony_id)
            .or_insert_with(Vec::new)
            .push((*job, ant.age));
    }

    // For each colony, compute desired ratios and compute input for reassignment logic
    for (colony_id, ant_data) in ants_by_colony {
        // Use default sliders if no slider component found
        let default_sliders = BehaviorSliders::default();
        let sliders = sliders_query.iter().next().unwrap_or(&default_sliders);

        // Count current job distribution
        let mut counts = JobCounts::default();
        for (job, _) in &ant_data {
            match job {
                AntJob::Forager => counts.forager += 1,
                AntJob::Nurse => counts.nurse += 1,
                AntJob::Digger => counts.digger += 1,
                AntJob::Defender => counts.defender += 1,
                AntJob::Unassigned => counts.unassigned += 1,
            }
        }

        let input = JobAssignmentInput {
            total_ants: ant_data.len(),
            target_ratios: JobRatios {
                forage: sliders.forage,
                nurse: sliders.nurse,
                dig: sliders.dig,
                defend: sliders.defend,
            },
            current_assignments: counts,
        };

        // Apply reassignments to ants in this colony
        for (mut job, ant, colony) in &mut ant_query {
            if colony.colony_id != colony_id {
                continue;
            }

            if let Some(new_job) = job_assignment::should_reassign_ant(*job, ant.age, &input, 0.05) {
                *job = new_job;
            }
        }
    }
}
