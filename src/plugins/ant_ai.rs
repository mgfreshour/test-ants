use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{Ant, AntState, CarriedItem, ColonyMember, Health, Movement};
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
                    ant_forage_steering,
                    ant_return_steering,
                    food_detection_and_pickup,
                    nest_food_deposit,
                    ant_movement,
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

/// Foraging ants: follow FOOD pheromone gradient or random walk, biased away from HOME
fn ant_forage_steering(
    clock: Res<SimClock>,
    config: Res<SimConfig>,
    phero_grid: Option<Res<PheromoneGrid>>,
    mut query: Query<(&Transform, &mut Movement, &Ant), Without<CarriedItem>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let mut rng = rand::thread_rng();
    let noise = config.exploration_noise;

    for (transform, mut movement, ant) in &mut query {
        if ant.state != AntState::Foraging {
            continue;
        }

        let pos = transform.translation.truncate();
        let mut pheromone_bias = Vec2::ZERO;

        if let Some(ref grid) = phero_grid {
            if let Some((gx, gy)) = grid.world_to_grid(pos) {
                // Follow food pheromone toward food sources
                let food_grad = grid.sense_gradient(gx, gy, PheromoneType::Food);
                if food_grad.length_squared() > 0.01 {
                    pheromone_bias += food_grad.normalize() * food_grad.length().min(10.0) * 0.05;
                }
                // Slight bias away from home pheromone (explore outward)
                let home_grad = grid.sense_gradient(gx, gy, PheromoneType::Home);
                if home_grad.length_squared() > 0.01 {
                    pheromone_bias -= home_grad.normalize() * 0.01;
                }
            }
        }

        let angle_offset = rng.gen_range(-noise..noise) * std::f32::consts::TAU;
        let current_angle = movement.direction.y.atan2(movement.direction.x);
        let new_angle = current_angle + angle_offset;
        let random_dir = Vec2::new(new_angle.cos(), new_angle.sin());

        let mut new_dir = (random_dir + pheromone_bias).normalize_or_zero();
        if new_dir == Vec2::ZERO {
            new_dir = random_dir;
        }
        movement.direction = new_dir;
    }
}

/// Returning ants: follow HOME pheromone gradient back to nest
fn ant_return_steering(
    clock: Res<SimClock>,
    config: Res<SimConfig>,
    phero_grid: Option<Res<PheromoneGrid>>,
    mut query: Query<(&Transform, &mut Movement, &Ant), With<CarriedItem>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let mut rng = rand::thread_rng();
    let noise = config.exploration_noise * 0.5; // less random when returning
    let nest_pos = config.nest_position;

    for (transform, mut movement, ant) in &mut query {
        if ant.state != AntState::Returning {
            continue;
        }

        let pos = transform.translation.truncate();
        let mut bias = Vec2::ZERO;

        if let Some(ref grid) = phero_grid {
            if let Some((gx, gy)) = grid.world_to_grid(pos) {
                let home_grad = grid.sense_gradient(gx, gy, PheromoneType::Home);
                if home_grad.length_squared() > 0.01 {
                    bias += home_grad.normalize() * home_grad.length().min(10.0) * 0.08;
                }
            }
        }

        // Fallback: direct vector toward nest
        let to_nest = nest_pos - pos;
        if to_nest.length_squared() > 1.0 {
            bias += to_nest.normalize() * 0.03;
        }

        let angle_offset = rng.gen_range(-noise..noise) * std::f32::consts::TAU;
        let current_angle = movement.direction.y.atan2(movement.direction.x);
        let new_angle = current_angle + angle_offset;
        let random_dir = Vec2::new(new_angle.cos(), new_angle.sin());

        let mut new_dir = (random_dir + bias).normalize_or_zero();
        if new_dir == Vec2::ZERO {
            new_dir = random_dir;
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
        (Entity, &Transform, &mut Ant),
        Without<CarriedItem>,
    >,
    mut food_query: Query<(&Transform, &mut FoodSource)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    for (ant_entity, ant_transform, mut ant) in &mut ant_query {
        if ant.state != AntState::Foraging {
            continue;
        }

        let ant_pos = ant_transform.translation.truncate();
        let mut picked_up = false;

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
                picked_up = true;
                break;
            }
        }

        let _ = picked_up;
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
        (Entity, &Transform, &mut Ant, &CarriedItem),
    >,
    _nest_query: Query<&Transform, With<NestEntrance>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let nest_pos = config.nest_position;

    for (ant_entity, ant_transform, mut ant, carried) in &mut ant_query {
        if ant.state != AntState::Returning {
            continue;
        }

        let ant_pos = ant_transform.translation.truncate();
        let dist = ant_pos.distance(nest_pos);

        if dist < NEST_DEPOSIT_RANGE {
            colony_food.stored += carried.food_amount;
            commands.entity(ant_entity).remove::<CarriedItem>();
            ant.state = AntState::Foraging;
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
                // Foragers leave HOME pheromone so returning ants can find the nest
                grid.deposit(gx, gy, PheromoneType::Home, pconfig.deposit_amount, pconfig.max_intensity);
            }
            AntState::Returning => {
                // Returners leave FOOD pheromone so foragers can find food
                let amount = if let Some(c) = carried {
                    pconfig.deposit_amount * (1.0 + c.food_amount * 0.1)
                } else {
                    pconfig.deposit_amount
                };
                grid.deposit(gx, gy, PheromoneType::Food, amount, pconfig.max_intensity);
            }
            _ => {}
        }
    }
}

fn ant_movement(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Movement), With<Ant>>,
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
    mut query: Query<(&Ant, &mut Sprite, Option<&CarriedItem>)>,
) {
    for (_ant, mut sprite, carried) in &mut query {
        if carried.is_some() {
            sprite.color = Color::srgb(0.15, 0.5, 0.15);
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
