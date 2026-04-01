use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{Ant, Caste, ColonyMember, Health, Movement, PositionHistory, TrailSense};
use crate::components::nest::{Brood, BroodStage, CellType, NestPath, NestTestAnt, NestTile, Queen};
use crate::plugins::ant_ai::ColonyFood;
use crate::resources::colony::{CasteRatios, ColonyStats};
use crate::resources::nest::{NestGrid, NEST_CELL_SIZE, NEST_HEIGHT, NEST_WIDTH};
use crate::resources::nest_pathfinding::NestPathCache;
use crate::resources::nest_pheromone::{
    LABEL_BROOD, LABEL_ENTRANCE, LABEL_FOOD_STORAGE, LABEL_MIDDEN, LABEL_QUEEN,
};
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};

pub struct NestPlugin;

#[derive(Component)]
pub struct NestViewEntity;

#[derive(Component)]
pub struct SurfaceViewEntity;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameView {
    #[default]
    Surface,
    Underground,
}

#[derive(Resource, Default)]
struct SavedSurfaceCamera {
    position: Vec2,
    scale: f32,
}

#[derive(Component)]
struct FoodStorageIndicator {
    index: usize,
}

const QUEEN_EGG_INTERVAL: f32 = 10.0;
const QUEEN_FOOD_COST: f32 = 2.0;

fn nest_grid_to_world(gx: usize, gy: usize) -> Vec2 {
    let offset_x = -(NEST_WIDTH as f32 * NEST_CELL_SIZE) / 2.0;
    let offset_y = (NEST_HEIGHT as f32 * NEST_CELL_SIZE) / 2.0;
    Vec2::new(
        offset_x + gx as f32 * NEST_CELL_SIZE + NEST_CELL_SIZE / 2.0,
        offset_y - gy as f32 * NEST_CELL_SIZE - NEST_CELL_SIZE / 2.0,
    )
}

impl Plugin for NestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NestGrid>()
            .init_resource::<ColonyStats>()
            .init_resource::<CasteRatios>()
            .init_resource::<SavedSurfaceCamera>()
            .init_state::<GameView>()
            .add_systems(Startup, (render_nest, spawn_queen, spawn_test_ants, spawn_food_indicators))
            .add_systems(Update, toggle_view)
            .add_systems(OnEnter(GameView::Underground), enter_underground)
            .add_systems(OnExit(GameView::Underground), exit_underground)
            .add_systems(
                Update,
                (
                    queen_egg_laying,
                    brood_development,
                    update_colony_stats,
                    update_food_indicators,
                ),
            )
            .add_systems(
                Update,
                test_ant_retarget.run_if(in_state(GameView::Underground)),
            );
    }
}

fn render_nest(mut commands: Commands, grid: Res<NestGrid>) {
    for y in 0..grid.height {
        for x in 0..grid.width {
            let cell = grid.get(x, y);
            let w = nest_grid_to_world(x, y);

            commands.spawn((
                Sprite {
                    color: cell.color(),
                    custom_size: Some(Vec2::splat(NEST_CELL_SIZE)),
                    ..default()
                },
                Transform::from_xyz(w.x, w.y, 0.0),
                Visibility::Hidden,
                NestTile { grid_x: x, grid_y: y },
                NestViewEntity,
            ));
        }
    }
}

fn spawn_queen(mut commands: Commands) {
    let cx = NEST_WIDTH / 2;
    let queen_center = nest_grid_to_world(cx, 16);

    commands.spawn((
        Sprite {
            color: Color::srgb(0.8, 0.6, 0.1),
            custom_size: Some(Vec2::splat(12.0)),
            ..default()
        },
        Transform::from_xyz(queen_center.x, queen_center.y, 3.0),
        Visibility::Hidden,
        Queen,
        NestViewEntity,
        Health { current: 100.0, max: 100.0 },
    ));
}

/// Spawn 8 test ants with hardcoded destinations for pathfinding verification.
fn spawn_test_ants(
    mut commands: Commands,
    grid: Res<NestGrid>,
    mut path_cache: ResMut<NestPathCache>,
) {
    let mut rng = rand::thread_rng();

    // Define test ant assignments: (start_chamber, target_label, color)
    // Each ant starts in one chamber and pathfinds to another.
    let assignments = [
        (LABEL_ENTRANCE, LABEL_BROOD, Color::srgb(0.2, 0.6, 0.2)),       // green: entrance -> brood
        (LABEL_ENTRANCE, LABEL_QUEEN, Color::srgb(0.6, 0.2, 0.6)),       // purple: entrance -> queen
        (LABEL_BROOD, LABEL_FOOD_STORAGE, Color::srgb(0.2, 0.5, 0.8)),   // blue: brood -> food
        (LABEL_FOOD_STORAGE, LABEL_BROOD, Color::srgb(0.8, 0.5, 0.2)),   // orange: food -> brood
        (LABEL_QUEEN, LABEL_ENTRANCE, Color::srgb(0.8, 0.8, 0.2)),       // yellow: queen -> entrance
        (LABEL_QUEEN, LABEL_MIDDEN, Color::srgb(0.5, 0.5, 0.5)),         // gray: queen -> midden
        (LABEL_MIDDEN, LABEL_QUEEN, Color::srgb(0.8, 0.3, 0.3)),         // red: midden -> queen
        (LABEL_FOOD_STORAGE, LABEL_QUEEN, Color::srgb(0.3, 0.8, 0.8)),   // cyan: food -> queen
    ];

    for &(start_label, target_label, color) in &assignments {
        // Find a passable cell in the start chamber
        let start = find_chamber_cell(&grid, start_label);
        let goal = find_chamber_cell(&grid, target_label);

        let (start_pos, goal_pos) = match (start, goal) {
            (Some(s), Some(g)) => (s, g),
            _ => continue,
        };

        let path = path_cache.find_path(&grid, start_pos, goal_pos);
        let world_pos = nest_grid_to_world(start_pos.0, start_pos.1);
        let jitter_x = rng.gen_range(-3.0..3.0);
        let jitter_y = rng.gen_range(-3.0..3.0);

        let mut entity = commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::splat(5.0)),
                ..default()
            },
            Transform::from_xyz(world_pos.x + jitter_x, world_pos.y + jitter_y, 2.5),
            Visibility::Hidden,
            NestViewEntity,
            NestTestAnt {
                target_label,
                retarget_timer: 0.0,
            },
        ));

        if let Some(waypoints) = path {
            entity.insert(NestPath::new(waypoints));
        }
    }
}

/// Find a passable cell belonging to a chamber identified by label index.
fn find_chamber_cell(grid: &NestGrid, label: usize) -> Option<(usize, usize)> {
    use crate::resources::nest_pheromone::chamber_kind_to_label;

    let cx = NEST_WIDTH / 2;

    // For entrance, return the top of the entrance tunnel
    if label == LABEL_ENTRANCE {
        for y in 0..grid.height {
            if grid.get(cx, y).is_passable() {
                return Some((cx, y));
            }
        }
        return None;
    }

    // For chambers, find a cell of the matching type
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

fn spawn_food_indicators(mut commands: Commands) {
    let cx = NEST_WIDTH / 2;
    let capacity = 12;
    let mut idx = 0;

    for gy in 5..8 {
        for gx in (cx - 5)..(cx - 1) {
            if idx >= capacity {
                break;
            }
            let pos = nest_grid_to_world(gx, gy);

            commands.spawn((
                Sprite {
                    color: Color::srgba(0.6, 0.8, 0.2, 0.0),
                    custom_size: Some(Vec2::splat(NEST_CELL_SIZE * 0.6)),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, 1.5),
                Visibility::Hidden,
                NestViewEntity,
                FoodStorageIndicator { index: idx },
            ));
            idx += 1;
        }
    }
}

fn collect_passable_cells(grid: &NestGrid) -> Vec<(usize, usize)> {
    let mut cells = Vec::new();
    for y in 0..grid.height {
        for x in 0..grid.width {
            let cell = grid.get(x, y);
            if matches!(cell, CellType::Tunnel | CellType::Chamber(_)) {
                cells.push((x, y));
            }
        }
    }
    cells
}

fn toggle_view(
    input: Res<ButtonInput<KeyCode>>,
    current: Res<State<GameView>>,
    mut next: ResMut<NextState<GameView>>,
) {
    if input.just_pressed(KeyCode::Tab) {
        match current.get() {
            GameView::Surface => next.set(GameView::Underground),
            GameView::Underground => next.set(GameView::Surface),
        }
    }
}

fn enter_underground(
    mut saved: ResMut<SavedSurfaceCamera>,
    mut camera_query: Query<(&mut Transform, &mut OrthographicProjection), With<crate::plugins::camera::MainCamera>>,
    mut nest_q: Query<&mut Visibility, With<NestViewEntity>>,
    mut neutral_q: Query<&mut Visibility, (Without<NestViewEntity>, Without<crate::plugins::camera::MainCamera>, Without<Node>)>,
) {
    if let Ok((mut cam_tf, mut proj)) = camera_query.get_single_mut() {
        saved.position = cam_tf.translation.truncate();
        saved.scale = proj.scale;
        cam_tf.translation.x = 0.0;
        cam_tf.translation.y = 0.0;
        proj.scale = 0.7;
    }

    for mut vis in &mut nest_q {
        *vis = Visibility::Visible;
    }
    for mut vis in &mut neutral_q {
        *vis = Visibility::Hidden;
    }
}

fn exit_underground(
    saved: Res<SavedSurfaceCamera>,
    mut camera_query: Query<(&mut Transform, &mut OrthographicProjection), With<crate::plugins::camera::MainCamera>>,
    mut nest_q: Query<&mut Visibility, With<NestViewEntity>>,
    mut neutral_q: Query<&mut Visibility, (Without<NestViewEntity>, Without<crate::plugins::camera::MainCamera>, Without<Node>)>,
) {
    if let Ok((mut cam_tf, mut proj)) = camera_query.get_single_mut() {
        cam_tf.translation.x = saved.position.x;
        cam_tf.translation.y = saved.position.y;
        proj.scale = saved.scale;
    }

    for mut vis in &mut nest_q {
        *vis = Visibility::Hidden;
    }
    for mut vis in &mut neutral_q {
        *vis = Visibility::Inherited;
    }
}

fn queen_egg_laying(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut commands: Commands,
    mut colony_food: ResMut<ColonyFood>,
    queen_query: Query<Entity, With<Queen>>,
    mut egg_timer: Local<f32>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }
    if queen_query.is_empty() {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    *egg_timer += dt;

    if *egg_timer >= QUEEN_EGG_INTERVAL && colony_food.stored >= QUEEN_FOOD_COST {
        *egg_timer -= QUEEN_EGG_INTERVAL;
        colony_food.stored -= QUEEN_FOOD_COST;

        let mut rng = rand::thread_rng();
        let cx = NEST_WIDTH / 2;
        let gx = rng.gen_range((cx + 2)..(cx + 7));
        let gy = rng.gen_range(8..12);
        let pos = nest_grid_to_world(gx, gy);
        let jitter = Vec2::new(rng.gen_range(-5.0..5.0), rng.gen_range(-5.0..5.0));

        commands.spawn((
            Sprite {
                color: Color::srgb(0.95, 0.95, 0.85),
                custom_size: Some(Vec2::splat(3.0)),
                ..default()
            },
            Transform::from_xyz(pos.x + jitter.x, pos.y + jitter.y, 2.5),
            Visibility::Hidden,
            Brood::new_egg(),
            NestViewEntity,
        ));
    }
}

fn brood_development(
    clock: Res<SimClock>,
    time: Res<Time>,
    config: Res<SimConfig>,
    caste_ratios: Res<CasteRatios>,
    view: Res<State<GameView>>,
    mut commands: Commands,
    mut brood_query: Query<(Entity, &mut Brood, &mut Sprite)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    let mut rng = rand::thread_rng();

    for (entity, mut brood, mut sprite) in &mut brood_query {
        brood.timer += dt;

        if brood.timer >= brood.stage_duration() {
            brood.timer = 0.0;
            match brood.stage {
                BroodStage::Egg => {
                    brood.stage = BroodStage::Larva;
                    sprite.color = Color::srgb(0.9, 0.85, 0.7);
                    sprite.custom_size = Some(Vec2::splat(4.0));
                }
                BroodStage::Larva => {
                    brood.stage = BroodStage::Pupa;
                    sprite.color = Color::srgb(0.7, 0.65, 0.5);
                    sprite.custom_size = Some(Vec2::splat(5.0));
                }
                BroodStage::Pupa => {
                    commands.entity(entity).despawn();

                    let nest = config.nest_position;
                    let caste = caste_ratios.pick_caste(rng.gen::<f32>());
                    let (speed, health, state, color) = match caste {
                        Caste::Worker => (
                            config.ant_speed_worker,
                            Health::worker(),
                            crate::components::ant::AntState::Foraging,
                            Color::srgb(0.1, 0.1, 0.1),
                        ),
                        Caste::Soldier => (
                            config.ant_speed_soldier,
                            Health::soldier(),
                            crate::components::ant::AntState::Defending,
                            Color::srgb(0.3, 0.1, 0.1),
                        ),
                        _ => (
                            config.ant_speed_worker,
                            Health::worker(),
                            crate::components::ant::AntState::Foraging,
                            Color::srgb(0.1, 0.1, 0.1),
                        ),
                    };

                    let offset_x = rng.gen_range(-15.0..15.0);
                    let offset_y = rng.gen_range(-15.0..15.0);

                    let vis = if *view.get() == GameView::Underground {
                        Visibility::Hidden
                    } else {
                        Visibility::Inherited
                    };

                    commands.spawn((
                        Sprite {
                            color,
                            custom_size: Some(Vec2::splat(4.0)),
                            ..default()
                        },
                        Transform::from_xyz(nest.x + offset_x, nest.y + offset_y, 2.0),
                        vis,
                        Ant {
                            caste,
                            state,
                            age: 0.0,
                            hunger: 0.0,
                        },
                        Movement::with_random_direction(speed, &mut rng),
                        health,
                        ColonyMember { colony_id: 0 },
                        PositionHistory::default(),
                        TrailSense::default(),
                    ));
                }
            }
        }
    }
}

fn update_colony_stats(
    mut stats: ResMut<ColonyStats>,
    ant_query: Query<&Ant>,
    brood_query: Query<&Brood>,
) {
    stats.workers = 0;
    stats.soldiers = 0;
    stats.drones = 0;
    stats.eggs = 0;
    stats.larvae = 0;
    stats.pupae = 0;

    for ant in &ant_query {
        match ant.caste {
            Caste::Worker => stats.workers += 1,
            Caste::Soldier => stats.soldiers += 1,
            Caste::Drone => stats.drones += 1,
            _ => {}
        }
    }

    for brood in &brood_query {
        match brood.stage {
            BroodStage::Egg => stats.eggs += 1,
            BroodStage::Larva => stats.larvae += 1,
            BroodStage::Pupa => stats.pupae += 1,
        }
    }
}

/// When test ants complete their path, pick a new random destination and compute a new path.
fn test_ant_retarget(
    clock: Res<SimClock>,
    time: Res<Time>,
    grid: Res<NestGrid>,
    mut path_cache: ResMut<NestPathCache>,
    mut query: Query<(&Transform, &mut NestTestAnt, &mut NestPath)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    let mut rng = rand::thread_rng();

    let labels = [LABEL_ENTRANCE, LABEL_BROOD, LABEL_FOOD_STORAGE, LABEL_QUEEN, LABEL_MIDDEN];

    for (transform, mut test_ant, mut path) in &mut query {
        if !path.is_complete() {
            continue;
        }

        test_ant.retarget_timer -= dt;
        if test_ant.retarget_timer > 0.0 {
            continue;
        }

        // Pick a new random destination different from current target
        let new_label = loop {
            let candidate = labels[rng.gen_range(0..labels.len())];
            if candidate != test_ant.target_label {
                break candidate;
            }
        };
        test_ant.target_label = new_label;
        test_ant.retarget_timer = rng.gen_range(1.0..3.0);

        // Find current grid position
        let pos = transform.translation.truncate();
        let current_grid = match crate::plugins::nest_navigation::world_to_nest_grid(pos) {
            Some(g) => g,
            None => continue,
        };

        // Find a cell in the target chamber
        let goal = match find_chamber_cell(&grid, new_label) {
            Some(g) => g,
            None => continue,
        };

        // Compute path
        if let Some(waypoints) = path_cache.find_path(&grid, current_grid, goal) {
            *path = NestPath::new(waypoints);
        }
    }
}

fn update_food_indicators(
    colony_food: Res<ColonyFood>,
    mut query: Query<(&FoodStorageIndicator, &mut Sprite)>,
) {
    let food = colony_food.stored;
    let food_per_slot = 5.0;

    for (indicator, mut sprite) in &mut query {
        let threshold = indicator.index as f32 * food_per_slot;
        if food > threshold + food_per_slot {
            sprite.color = Color::srgba(0.55, 0.75, 0.15, 0.85);
        } else if food > threshold {
            let frac = (food - threshold) / food_per_slot;
            sprite.color = Color::srgba(0.55, 0.75, 0.15, frac * 0.85);
        } else {
            sprite.color = Color::srgba(0.55, 0.75, 0.15, 0.0);
        }
    }
}
