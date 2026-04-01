use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{Ant, AntState, CarriedItem, ColonyMember, Follower, Health, Movement, PlayerControlled, PositionHistory};
use crate::components::pheromone::PheromoneType;
use crate::components::terrain::{FoodSource, NestEntrance};
use crate::resources::pheromone::{PheromoneConfig, PheromoneGrid};
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};
use crate::resources::spatial_grid::SpatialGrid;

pub struct AntAiPlugin;

#[derive(Resource, Default)]
pub struct ColonyFood {
    pub stored: f32,
}

impl Plugin for AntAiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpatialGrid>()
            .init_resource::<ColonyFood>()
            .add_systems(Startup, spawn_initial_ants)
            .add_systems(
                Update,
                (
                    rebuild_spatial_grid,
                    fix_orphaned_returners,
                    ant_forage_steering,
                    ant_return_steering,
                    food_detection_and_pickup,
                    nest_food_deposit,
                    ant_movement,
                    record_position_history,
                    ant_boundary_bounce,
                    ant_pheromone_deposit,
                    update_ant_visuals,
                    food_depletion_cleanup,
                )
                    .chain(),
            );
    }
}

fn spawn_initial_ants(mut commands: Commands, config: Res<SimConfig>) {
    let mut rng = rand::thread_rng();
    let nest = config.nest_position;

    for _ in 0..config.initial_ant_count {
        let offset_x = rng.gen_range(-20.0..20.0);
        let offset_y = rng.gen_range(-20.0..20.0);

        commands.spawn((
            Sprite {
                color: Color::srgb(0.1, 0.1, 0.1),
                custom_size: Some(Vec2::splat(4.0)),
                ..default()
            },
            Transform::from_xyz(nest.x + offset_x, nest.y + offset_y, 2.0),
            Ant::new_worker(),
            Movement::with_random_direction(config.ant_speed_worker, &mut rng),
            Health::worker(),
            ColonyMember { colony_id: 0 },
            PositionHistory::default(),
        ));
    }
}

fn rebuild_spatial_grid(
    mut grid: ResMut<SpatialGrid>,
    query: Query<(Entity, &Transform), With<Ant>>,
) {
    grid.clear();
    for (entity, transform) in &query {
        grid.insert(entity, transform.translation.truncate());
    }
}

const ANTI_BACKTRACK_WEIGHT: f32 = 0.35;
const FORWARD_WEIGHT: f32 = 0.6;
const SENSE_RANGE: f32 = 60.0;
const PHERO_SENSE_RADIUS: i32 = 4;
const PHERO_TRAIL_WEIGHT: f32 = 1.5;
/// Probability an ant follows a detected pheromone trail vs. ignoring it to scout
const TRAIL_FOLLOW_CHANCE: f32 = 0.6;

/// Reset ants stuck in Returning state without food back to Foraging.
fn fix_orphaned_returners(
    mut query: Query<(&mut Ant, &mut PositionHistory), (Without<CarriedItem>, Without<PlayerControlled>)>,
) {
    for (mut ant, mut history) in &mut query {
        if ant.state == AntState::Returning {
            ant.state = AntState::Foraging;
            history.clear();
        }
    }
}

/// Foraging ants: follow FOOD pheromone gradient or random walk, biased away from HOME.
/// Within SENSE_RANGE of a food source, head straight for it.
fn ant_forage_steering(
    clock: Res<SimClock>,
    config: Res<SimConfig>,
    phero_grid: Option<Res<PheromoneGrid>>,
    food_query: Query<&Transform, With<FoodSource>>,
    mut query: Query<(&Transform, &mut Movement, &Ant, &PositionHistory), (Without<CarriedItem>, Without<PlayerControlled>, Without<Follower>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let mut rng = rand::thread_rng();
    let noise = config.exploration_noise;

    for (transform, mut movement, ant, history) in &mut query {
        if ant.state != AntState::Foraging {
            continue;
        }

        let pos = transform.translation.truncate();
        let fwd = movement.direction;

        // Check for nearby food — override all other steering
        let mut nearest_food: Option<(f32, Vec2)> = None;
        for food_tf in &food_query {
            let food_pos = food_tf.translation.truncate();
            let dist = pos.distance(food_pos);
            if dist < SENSE_RANGE {
                if nearest_food.is_none() || dist < nearest_food.unwrap().0 {
                    nearest_food = Some((dist, food_pos));
                }
            }
        }

        if let Some((_, food_pos)) = nearest_food {
            let to_food = (food_pos - pos).normalize_or_zero();
            movement.direction = to_food;
            continue;
        }

        let mut pheromone_bias = Vec2::ZERO;
        let mut on_trail = false;

        if let Some(ref grid) = phero_grid {
            if let Some((gx, gy)) = grid.world_to_grid(pos) {
                let food_grad =
                    grid.sense_gradient(gx, gy, PheromoneType::Food, fwd, PHERO_SENSE_RADIUS);
                if food_grad.length_squared() > 0.01
                    && rng.gen::<f32>() < TRAIL_FOLLOW_CHANCE
                {
                    let fg = food_grad.normalize();
                    let along = fwd.dot(fg) * fwd;
                    let lateral = fg - along;
                    if lateral.length_squared() > 0.001 {
                        pheromone_bias += lateral.normalize() * PHERO_TRAIL_WEIGHT;
                    }
                    on_trail = true;
                }
            }
        }

        let angle_offset = rng.gen_range(-noise..noise) * std::f32::consts::TAU;
        let current_angle = fwd.y.atan2(fwd.x);
        let new_angle = current_angle + angle_offset;
        let perturbed_fwd = Vec2::new(new_angle.cos(), new_angle.sin());

        let momentum = history.anti_backtrack(pos) * ANTI_BACKTRACK_WEIGHT;

        // When on a pheromone trail, reduce random noise so the ant commits to following it
        let noise_scale = if on_trail { 0.3 } else { 1.0 };
        let mut new_dir = (fwd * FORWARD_WEIGHT
            + perturbed_fwd * noise_scale
            + pheromone_bias
            + momentum)
            .normalize_or_zero();
        if new_dir == Vec2::ZERO {
            new_dir = perturbed_fwd;
        }
        movement.direction = new_dir;
    }
}

/// Returning ants: follow HOME pheromone gradient back to nest.
/// Within SENSE_RANGE of the nest, head straight for it.
fn ant_return_steering(
    clock: Res<SimClock>,
    config: Res<SimConfig>,
    phero_grid: Option<Res<PheromoneGrid>>,
    mut query: Query<(&Transform, &mut Movement, &Ant, &PositionHistory), (With<CarriedItem>, Without<PlayerControlled>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let mut rng = rand::thread_rng();
    let noise = config.exploration_noise * 0.5;
    let nest_pos = config.nest_position;

    for (transform, mut movement, ant, history) in &mut query {
        if ant.state != AntState::Returning {
            continue;
        }

        let pos = transform.translation.truncate();
        let to_nest = nest_pos - pos;
        let dist_to_nest = to_nest.length();

        // Close enough to sense the nest directly — beeline for it
        if dist_to_nest < SENSE_RANGE {
            movement.direction = to_nest.normalize_or_zero();
            continue;
        }

        let fwd = movement.direction;
        let mut bias = Vec2::ZERO;

        let mut on_trail = false;

        if let Some(ref grid) = phero_grid {
            if let Some((gx, gy)) = grid.world_to_grid(pos) {
                let home_grad =
                    grid.sense_gradient(gx, gy, PheromoneType::Home, fwd, PHERO_SENSE_RADIUS);
                if home_grad.length_squared() > 0.01 {
                    bias += home_grad.normalize() * PHERO_TRAIL_WEIGHT;
                    on_trail = true;
                }
            }
        }

        if dist_to_nest > 1.0 {
            bias += to_nest.normalize() * 0.03;
        }

        let angle_offset = rng.gen_range(-noise..noise) * std::f32::consts::TAU;
        let current_angle = fwd.y.atan2(fwd.x);
        let new_angle = current_angle + angle_offset;
        let perturbed_fwd = Vec2::new(new_angle.cos(), new_angle.sin());

        let momentum = history.anti_backtrack(pos) * ANTI_BACKTRACK_WEIGHT;

        let noise_scale = if on_trail { 0.3 } else { 1.0 };
        let mut new_dir = (fwd * FORWARD_WEIGHT + perturbed_fwd * noise_scale + bias + momentum)
            .normalize_or_zero();
        if new_dir == Vec2::ZERO {
            new_dir = perturbed_fwd;
        }
        movement.direction = new_dir;
    }
}

const FOOD_PICKUP_RANGE: f32 = 20.0;

/// Foraging ants near a food source pick it up
fn food_detection_and_pickup(
    clock: Res<SimClock>,
    mut commands: Commands,
    mut ant_query: Query<
        (Entity, &Transform, &mut Ant, &mut PositionHistory),
        Without<CarriedItem>,
    >,
    mut food_query: Query<(&Transform, &mut FoodSource)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    for (ant_entity, ant_transform, mut ant, mut history) in &mut ant_query {
        if ant.state != AntState::Foraging {
            continue;
        }

        let ant_pos = ant_transform.translation.truncate();

        for (food_transform, mut food) in &mut food_query {
            if food.remaining <= 0.0 {
                continue;
            }
            let food_pos = food_transform.translation.truncate();
            let dist = ant_pos.distance(food_pos);

            if dist < FOOD_PICKUP_RANGE {
                let amount = food.remaining.min(5.0);
                food.remaining -= amount;
                commands.entity(ant_entity).insert(CarriedItem { food_amount: amount });
                ant.state = AntState::Returning;
                history.clear();
                break;
            }
        }
    }
}

const NEST_DEPOSIT_RANGE: f32 = 30.0;

/// Returning ants near the nest deposit food
fn nest_food_deposit(
    clock: Res<SimClock>,
    mut commands: Commands,
    config: Res<SimConfig>,
    mut colony_food: ResMut<ColonyFood>,
    mut ant_query: Query<
        (Entity, &Transform, &mut Ant, &CarriedItem, &mut PositionHistory),
    >,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let nest_pos = config.nest_position;

    for (ant_entity, ant_transform, mut ant, carried, mut history) in &mut ant_query {
        if ant.state != AntState::Returning {
            continue;
        }

        let ant_pos = ant_transform.translation.truncate();
        let dist = ant_pos.distance(nest_pos);

        if dist < NEST_DEPOSIT_RANGE {
            colony_food.stored += carried.food_amount;
            commands.entity(ant_entity).remove::<CarriedItem>();
            ant.state = AntState::Foraging;
            history.clear();
        }
    }
}

/// Ants deposit pheromones based on state
fn ant_pheromone_deposit(
    clock: Res<SimClock>,
    pconfig: Res<PheromoneConfig>,
    mut grid: Option<ResMut<PheromoneGrid>>,
    query: Query<(&Transform, &Ant, Option<&CarriedItem>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let Some(ref mut grid) = grid else { return };

    for (transform, ant, carried) in &query {
        let pos = transform.translation.truncate();
        let Some((gx, gy)) = grid.world_to_grid(pos) else {
            continue;
        };

        match ant.state {
            AntState::Foraging => {
                let amt = pconfig.deposit_amount(PheromoneType::Home);
                grid.deposit(gx, gy, PheromoneType::Home, amt, pconfig.max_intensity);
            }
            AntState::Returning => {
                let base = pconfig.deposit_amount(PheromoneType::Food);
                let amt = if let Some(c) = carried {
                    base * (1.0 + c.food_amount * 0.1)
                } else {
                    base
                };
                grid.deposit(gx, gy, PheromoneType::Food, amt, pconfig.max_intensity);
            }
            _ => {}
        }
    }
}

fn ant_movement(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Movement), (With<Ant>, Without<PlayerControlled>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();

    for (mut transform, movement) in &mut query {
        let velocity = movement.direction * movement.speed * dt;
        transform.translation.x += velocity.x;
        transform.translation.y += velocity.y;
    }
}

fn record_position_history(
    clock: Res<SimClock>,
    mut query: Query<(&Transform, &mut PositionHistory), With<Ant>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    for (transform, mut history) in &mut query {
        history.record(transform.translation.truncate());
    }
}

fn ant_boundary_bounce(
    clock: Res<SimClock>,
    config: Res<SimConfig>,
    mut query: Query<(&mut Transform, &mut Movement), With<Ant>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let margin = 8.0;
    let min_x = margin;
    let max_x = config.world_width - margin;
    let min_y = margin;
    let max_y = config.world_height - margin;

    for (mut transform, mut movement) in &mut query {
        let pos = &mut transform.translation;

        if pos.x <= min_x {
            pos.x = min_x;
            movement.direction.x = movement.direction.x.abs();
        } else if pos.x >= max_x {
            pos.x = max_x;
            movement.direction.x = -movement.direction.x.abs();
        }

        if pos.y <= min_y {
            pos.y = min_y;
            movement.direction.y = movement.direction.y.abs();
        } else if pos.y >= max_y {
            pos.y = max_y;
            movement.direction.y = -movement.direction.y.abs();
        }
    }
}

/// Tint ants based on state: dark = foraging, green-tinted = carrying food
fn update_ant_visuals(
    mut query: Query<(&Ant, &mut Sprite, Option<&CarriedItem>), Without<PlayerControlled>>,
) {
    for (_ant, mut sprite, carried) in &mut query {
        if carried.is_some() {
            sprite.color = Color::srgb(0.9, 0.4, 0.1);
        } else {
            sprite.color = Color::srgb(0.1, 0.1, 0.1);
        }
    }
}

/// Despawn fully depleted food sources
fn food_depletion_cleanup(
    mut commands: Commands,
    query: Query<(Entity, &FoodSource)>,
) {
    for (entity, food) in &query {
        if food.remaining <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
