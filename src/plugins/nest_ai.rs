use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{Ant, AntState, ColonyMember, Health, PlayerControlled, Underground};
use crate::components::nest::{
    AttendStep, Brood, BroodStage, CellType, FeedStep, HaulStep, NestPath, NestTask, Queen,
};
use crate::plugins::ant_ai::ColonyFood;
use crate::plugins::nest::{GameView, NestViewEntity};
use crate::plugins::nest_navigation::{nest_grid_to_world, world_to_nest_grid};
use crate::resources::colony::BehaviorSliders;
use crate::resources::nest::{NestGrid, NEST_WIDTH};
use crate::resources::nest_pathfinding::NestPathCache;
use crate::resources::nest_pheromone::{
    NestPheromoneGrid, LABEL_BROOD, LABEL_ENTRANCE, LABEL_FOOD_STORAGE, LABEL_QUEEN,
};
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};

pub struct NestAiPlugin;

impl Plugin for NestAiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BehaviorSliders>()
            .add_systems(Startup, spawn_initial_nest_ants)
            .add_systems(
                Update,
                (
                    apply_brood_fed,
                    surface_to_nest_transition,
                    nest_to_surface_transition,
                    nest_utility_scoring,
                    nest_task_advance,
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

// ── Constants ──────────────────────────────────────────────────────────

/// How close an ant needs to be to a chamber cell to count as "arrived".
const ARRIVAL_THRESHOLD: f32 = 12.0;

/// How often nest ants re-evaluate their task (seconds).
const REEVALUATE_INTERVAL: f32 = 2.0;

/// Range at which surface ants detect the nest entrance for transition.
const NEST_ENTER_RANGE: f32 = 25.0;

/// Number of initial underground ants spawned at startup.
const INITIAL_NEST_ANTS: usize = 12;

// ── Spawn initial underground ants ────────────────────────────────────

fn spawn_initial_nest_ants(mut commands: Commands, grid: Res<NestGrid>) {
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
            NestViewEntity,
            Ant {
                caste: crate::components::ant::Caste::Worker,
                state: AntState::Nursing, // will be reassigned by utility AI
                age,
                hunger: 0.0,
            },
            Health::worker(),
            ColonyMember { colony_id: 0 },
            Underground,
            NestTask::Idle { timer: 0.0 },
        ));
    }
}

// ── Surface ↔ Nest Transitions ────────────────────────────────────────

/// Surface ants near the nest entrance may transition underground
/// based on the behavior sliders (nurse percentage governs how many go in).
fn surface_to_nest_transition(
    clock: Res<SimClock>,
    config: Res<SimConfig>,
    sliders: Res<BehaviorSliders>,
    grid: Res<NestGrid>,
    mut commands: Commands,
    mut query: Query<
        (Entity, &Transform, &mut Ant, &ColonyMember, &mut Visibility),
        (Without<Underground>, Without<PlayerControlled>),
    >,
    underground_count: Query<(), With<Underground>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let nest_pos = config.nest_position;
    let total_ants = query.iter().count() + underground_count.iter().count();
    let desired_underground = ((sliders.nurse + sliders.dig) * total_ants as f32).ceil() as usize;
    let current_underground = underground_count.iter().count();

    if current_underground >= desired_underground {
        return;
    }

    let mut rng = rand::thread_rng();

    for (entity, transform, mut ant, colony, mut vis) in &mut query {
        if colony.colony_id != 0 {
            continue;
        }
        if current_underground >= desired_underground {
            break;
        }

        let pos = transform.translation.truncate();
        let dist = pos.distance(nest_pos);

        if dist < NEST_ENTER_RANGE && ant.state == AntState::Foraging {
            // Small random chance per frame to prevent all entering at once
            if rng.gen::<f32>() > 0.02 {
                continue;
            }

            // Transition to underground
            ant.state = AntState::Nursing;
            *vis = Visibility::Hidden;

            // Find the entrance cell and place ant there
            let entrance = find_label_cell(&grid, LABEL_ENTRANCE);
            if let Some((_gx, _gy)) = entrance {
                commands.entity(entity).insert((
                    Underground,
                    NestViewEntity,
                    NestTask::Idle { timer: 0.0 },
                ));
            } else {
                commands.entity(entity).insert((
                    Underground,
                    NestViewEntity,
                    NestTask::Idle { timer: 0.0 },
                ));
            }
        }
    }
}

/// Nest ants that are idle for too long and are older exit to surface.
fn nest_to_surface_transition(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<
        (Entity, &mut Ant, &mut NestTask, &mut Visibility),
        With<Underground>,
    >,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();

    for (entity, mut ant, mut task, mut vis) in &mut query {
        if let NestTask::Idle { ref mut timer } = *task {
            *timer += dt;
            // Older ants (age > 200) exit after being idle for a few seconds
            if ant.age > 200.0 && *timer > 5.0 {
                ant.state = AntState::Foraging;
                *vis = Visibility::Inherited;

                // Move to nest entrance on surface
                commands
                    .entity(entity)
                    .remove::<Underground>()
                    .remove::<NestTask>()
                    .remove::<NestPath>()
                    .remove::<NestViewEntity>();

                // Reposition at nest entrance on surface
                // (position will be handled by surface systems via nest_position)
            }
        }
    }
}

// ── Utility AI Scoring ────────────────────────────────────────────────

/// Evaluate candidate actions for each underground ant and assign the best task.
fn nest_utility_scoring(
    clock: Res<SimClock>,
    time: Res<Time>,
    phero_grid: Res<NestPheromoneGrid>,
    colony_food: Res<ColonyFood>,
    brood_query: Query<&Brood>,
    queen_query: Query<(), With<Queen>>,
    mut query: Query<
        (Entity, &Transform, &Ant, &mut NestTask),
        With<Underground>,
    >,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let has_queen = !queen_query.is_empty();
    let has_food = colony_food.stored > 0.5;
    let unfed_larvae = brood_query
        .iter()
        .filter(|b| b.stage == BroodStage::Larva && !b.fed)
        .count();

    for (_entity, transform, ant, mut task) in &mut query {
        // Only re-evaluate when current task is complete or idle
        let should_reevaluate = match &*task {
            NestTask::Idle { timer } => *timer > REEVALUATE_INTERVAL,
            NestTask::FeedLarva { step, .. } => *step == FeedStep::DeliverFood,
            NestTask::HaulFood { step } => *step == HaulStep::DropFood,
            NestTask::AttendQueen { step } => *step == AttendStep::Grooming,
        };

        if !should_reevaluate {
            continue;
        }

        let pos = transform.translation.truncate();
        let grid_pos = world_to_nest_grid(pos);

        // Read pheromone inputs at current position
        let (brood_need, queen_signal) = if let Some((gx, gy)) = grid_pos {
            let cell = phero_grid.get(gx, gy);
            (cell.brood_need, cell.queen_signal)
        } else {
            (0.0, 0.0)
        };

        // Age-based affinity (temporal polyethism)
        let age_frac = (ant.age / 300.0).clamp(0.0, 1.0); // 0 = young, 1 = old
        let nursing_affinity = 1.0 - age_frac * 0.8; // young: 1.0, old: 0.2
        let hauling_affinity = 0.3 + age_frac * 0.7; // young: 0.3, old: 1.0
        let queen_affinity = 0.5; // constant

        // Score FEED_LARVA
        let feed_score = if unfed_larvae > 0 && has_food {
            let need = (unfed_larvae as f32 / 5.0).min(1.0);
            need * nursing_affinity * (0.3 + brood_need * 0.7)
        } else {
            0.0
        };

        // Score HAUL_FOOD (food at entrance waiting to be stored)
        let haul_score = if colony_food.stored > 2.0 {
            0.4 * hauling_affinity
        } else {
            0.0
        };

        // Score ATTEND_QUEEN
        let queen_score = if has_queen {
            0.3 * queen_affinity * (0.2 + queen_signal * 0.8)
        } else {
            0.0
        };

        // Score IDLE (always available as fallback)
        let idle_score = 0.05;

        // Pick highest scoring action
        let max_score = feed_score.max(haul_score).max(queen_score).max(idle_score);

        *task = if max_score == feed_score && feed_score > 0.0 {
            NestTask::FeedLarva {
                step: FeedStep::GoToStorage,
                target_larva: None,
            }
        } else if max_score == haul_score && haul_score > 0.0 {
            NestTask::HaulFood {
                step: HaulStep::GoToEntrance,
            }
        } else if max_score == queen_score && queen_score > 0.0 {
            NestTask::AttendQueen {
                step: AttendStep::GoToQueen,
            }
        } else {
            NestTask::Idle { timer: 0.0 }
        };
    }
}

// ── Task Chain Execution ──────────────────────────────────────────────

/// Advance task chain sub-steps: request pathfind, follow path, perform action.
fn nest_task_advance(
    clock: Res<SimClock>,
    time: Res<Time>,
    grid: Res<NestGrid>,
    mut path_cache: ResMut<NestPathCache>,
    mut colony_food: ResMut<ColonyFood>,
    mut commands: Commands,
    brood_query: Query<(Entity, &Transform, &Brood)>,
    _queen_query: Query<&Transform, With<Queen>>,
    mut ant_query: Query<
        (Entity, &Transform, &mut NestTask, Option<&NestPath>),
        With<Underground>,
    >,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();

    for (entity, transform, mut task, path) in &mut ant_query {
        let pos = transform.translation.truncate();
        let grid_pos = match world_to_nest_grid(pos) {
            Some(gp) => gp,
            None => continue,
        };

        let path_complete = path.map_or(true, |p| p.is_complete());

        match &mut *task {
            NestTask::FeedLarva { step, target_larva } => {
                match step {
                    FeedStep::GoToStorage => {
                        if path_complete {
                            // Request path to food storage
                            if let Some(goal) = find_label_cell(&grid, LABEL_FOOD_STORAGE) {
                                if let Some(waypoints) = path_cache.find_path(&grid, grid_pos, goal) {
                                    commands.entity(entity).insert(NestPath::new(waypoints));
                                }
                            }
                            *step = FeedStep::PickUpFood;
                        }
                    }
                    FeedStep::PickUpFood => {
                        if path_complete {
                            // "Pick up" food from colony storage
                            if colony_food.stored >= 1.0 {
                                colony_food.stored -= 1.0;
                                *step = FeedStep::GoToBrood;
                            } else {
                                // No food available, go idle
                                *task = NestTask::Idle { timer: 0.0 };
                                continue;
                            }
                        }
                    }
                    FeedStep::GoToBrood => {
                        if path_complete {
                            // Request path to brood chamber
                            if let Some(goal) = find_label_cell(&grid, LABEL_BROOD) {
                                if let Some(waypoints) = path_cache.find_path(&grid, grid_pos, goal) {
                                    commands.entity(entity).insert(NestPath::new(waypoints));
                                }
                            }
                            *step = FeedStep::FindLarva;
                        }
                    }
                    FeedStep::FindLarva => {
                        if path_complete {
                            // Find nearest unfed larva
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
                                // No unfed larvae found, task complete
                                *step = FeedStep::DeliverFood;
                            }
                        }
                    }
                    FeedStep::DeliverFood => {
                        // Feed the larva
                        if let Some(larva_entity) = target_larva {
                            commands.entity(*larva_entity).try_insert(BroodFed);
                        }
                        // Task complete — will be re-evaluated next cycle
                    }
                }
            }

            NestTask::HaulFood { step } => {
                match step {
                    HaulStep::GoToEntrance => {
                        if path_complete {
                            if let Some(goal) = find_label_cell(&grid, LABEL_ENTRANCE) {
                                if let Some(waypoints) = path_cache.find_path(&grid, grid_pos, goal) {
                                    commands.entity(entity).insert(NestPath::new(waypoints));
                                }
                            }
                            *step = HaulStep::PickUpFood;
                        }
                    }
                    HaulStep::PickUpFood => {
                        if path_complete {
                            // Hauler "picks up" food at entrance (already in colony_food)
                            *step = HaulStep::GoToStorage;
                        }
                    }
                    HaulStep::GoToStorage => {
                        if path_complete {
                            if let Some(goal) = find_label_cell(&grid, LABEL_FOOD_STORAGE) {
                                if let Some(waypoints) = path_cache.find_path(&grid, grid_pos, goal) {
                                    commands.entity(entity).insert(NestPath::new(waypoints));
                                }
                            }
                            *step = HaulStep::DropFood;
                        }
                    }
                    HaulStep::DropFood => {
                        // Task complete
                    }
                }
            }

            NestTask::AttendQueen { step } => {
                match step {
                    AttendStep::GoToQueen => {
                        if path_complete {
                            if let Some(goal) = find_label_cell(&grid, LABEL_QUEEN) {
                                if let Some(waypoints) = path_cache.find_path(&grid, grid_pos, goal) {
                                    commands.entity(entity).insert(NestPath::new(waypoints));
                                }
                            }
                            *step = AttendStep::Grooming;
                        }
                    }
                    AttendStep::Grooming => {
                        // Grooming continues until utility re-evaluates
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

// ── Task Labels ───────────────────────────────────────────────────────

#[derive(Component)]
struct NestTaskLabel;

/// Show task letter above each underground ant in the nest view.
fn nest_task_labels(
    view: Res<State<GameView>>,
    mut commands: Commands,
    ant_query: Query<(Entity, &NestTask, Option<&Children>), With<Underground>>,
    existing_labels: Query<Entity, With<NestTaskLabel>>,
) {
    // Clean up old labels
    for entity in &existing_labels {
        commands.entity(entity).despawn();
    }

    if *view.get() != GameView::Underground {
        return;
    }

    for (entity, task, _children) in &ant_query {
        // Don't spawn if entity already has child labels from surface system
        // (just spawn new floating label entities near the ant)
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
                NestViewEntity,
            ))
            .id();

        commands.entity(entity).add_child(label_entity);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

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
