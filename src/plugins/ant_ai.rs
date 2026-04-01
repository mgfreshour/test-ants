use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{Ant, ColonyMember, Health, Movement};
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};
use crate::resources::spatial_grid::SpatialGrid;

pub struct AntAiPlugin;

impl Plugin for AntAiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpatialGrid>()
            .add_systems(Startup, spawn_initial_ants)
            .add_systems(
                Update,
                (
                    rebuild_spatial_grid,
                    ant_random_walk,
                    ant_movement,
                    ant_boundary_bounce,
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

fn ant_random_walk(
    clock: Res<SimClock>,
    config: Res<SimConfig>,
    mut query: Query<&mut Movement, With<Ant>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let mut rng = rand::thread_rng();
    let noise = config.exploration_noise;

    for mut movement in &mut query {
        let angle_offset = rng.gen_range(-noise..noise) * std::f32::consts::TAU;
        let current_angle = movement.direction.y.atan2(movement.direction.x);
        let new_angle = current_angle + angle_offset;
        movement.direction = Vec2::new(new_angle.cos(), new_angle.sin());
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
