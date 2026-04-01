use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{Ant, Caste, ColonyMember, Health, Movement, PositionHistory};
use crate::components::nest::{Brood, BroodStage, NestTile, Queen};
use crate::plugins::ant_ai::ColonyFood;
use crate::resources::colony::{CasteRatios, ColonyStats};
use crate::resources::nest::{NestGrid, NEST_CELL_SIZE, NEST_HEIGHT, NEST_WIDTH};
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};

pub struct NestPlugin;

/// Marker for entities that only show in the nest (underground) view
#[derive(Component)]
pub struct NestViewEntity;

/// Marker for entities that only show on the surface view
#[derive(Component)]
pub struct SurfaceViewEntity;

#[derive(Resource)]
pub struct ViewState {
    pub underground: bool,
}

impl Default for ViewState {
    fn default() -> Self {
        Self { underground: false }
    }
}

const QUEEN_EGG_INTERVAL: f32 = 10.0;
const QUEEN_FOOD_COST: f32 = 2.0;

impl Plugin for NestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NestGrid>()
            .init_resource::<ViewState>()
            .init_resource::<ColonyStats>()
            .init_resource::<CasteRatios>()
            .add_systems(Startup, (render_nest, spawn_queen))
            .add_systems(
                Update,
                (
                    toggle_view,
                    sync_view_visibility,
                    queen_egg_laying,
                    brood_development,
                    update_colony_stats,
                )
                    .chain(),
            );
    }
}

fn render_nest(mut commands: Commands, grid: Res<NestGrid>) {
    let offset_x = -(NEST_WIDTH as f32 * NEST_CELL_SIZE) / 2.0;
    let offset_y = (NEST_HEIGHT as f32 * NEST_CELL_SIZE) / 2.0;

    for y in 0..grid.height {
        for x in 0..grid.width {
            let cell = grid.get(x, y);
            let wx = offset_x + x as f32 * NEST_CELL_SIZE + NEST_CELL_SIZE / 2.0;
            let wy = offset_y - y as f32 * NEST_CELL_SIZE - NEST_CELL_SIZE / 2.0;

            commands.spawn((
                Sprite {
                    color: cell.color(),
                    custom_size: Some(Vec2::splat(NEST_CELL_SIZE)),
                    ..default()
                },
                Transform::from_xyz(wx, wy, 0.0),
                Visibility::Hidden,
                NestTile { grid_x: x, grid_y: y },
                NestViewEntity,
            ));
        }
    }
}

fn spawn_queen(mut commands: Commands) {
    commands.spawn((
        Sprite {
            color: Color::srgb(0.8, 0.6, 0.1),
            custom_size: Some(Vec2::splat(10.0)),
            ..default()
        },
        Transform::from_xyz(0.0, -200.0, 3.0),
        Visibility::Hidden,
        Queen,
        NestViewEntity,
        Health { current: 100.0, max: 100.0 },
    ));
}

fn toggle_view(
    input: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ViewState>,
    mut camera_query: Query<&mut Transform, With<crate::plugins::camera::MainCamera>>,
) {
    if input.just_pressed(KeyCode::Tab) {
        state.underground = !state.underground;
        if let Ok(mut cam) = camera_query.get_single_mut() {
            if state.underground {
                cam.translation.x = 0.0;
                cam.translation.y = 0.0;
            } else {
                cam.translation.x = 1024.0;
                cam.translation.y = 1024.0;
            }
        }
    }
}

fn sync_view_visibility(
    state: Res<ViewState>,
    mut nest_q: Query<&mut Visibility, (With<NestViewEntity>, Without<SurfaceViewEntity>)>,
    mut surface_q: Query<&mut Visibility, (With<SurfaceViewEntity>, Without<NestViewEntity>)>,
    mut neutral_q: Query<&mut Visibility, (Without<NestViewEntity>, Without<SurfaceViewEntity>, Without<crate::plugins::camera::MainCamera>)>,
) {
    if !state.is_changed() {
        return;
    }

    for mut vis in &mut nest_q {
        *vis = if state.underground {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    for mut vis in &mut surface_q {
        *vis = if state.underground {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }

    for mut vis in &mut neutral_q {
        *vis = if state.underground {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

#[derive(Resource)]
struct EggTimer(f32);

impl Default for EggTimer {
    fn default() -> Self {
        Self(0.0)
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

        commands.spawn((
            Sprite {
                color: Color::srgb(0.95, 0.95, 0.85),
                custom_size: Some(Vec2::splat(3.0)),
                ..default()
            },
            Transform::from_xyz(20.0, -120.0, 2.5),
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
                    // Hatch into adult ant on the surface
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

                    commands.spawn((
                        Sprite {
                            color,
                            custom_size: Some(Vec2::splat(4.0)),
                            ..default()
                        },
                        Transform::from_xyz(nest.x + offset_x, nest.y + offset_y, 2.0),
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
