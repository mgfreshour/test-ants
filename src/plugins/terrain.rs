use bevy::prelude::*;
use rand::Rng;

use crate::components::terrain::{FoodSource, NestEntrance};
use crate::resources::simulation::SimConfig;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_terrain, spawn_nest_entrance, spawn_food_sources));
    }
}

fn setup_terrain(mut commands: Commands, config: Res<SimConfig>) {
    let tile_count_x = (config.world_width / config.tile_size) as i32;
    let tile_count_y = (config.world_height / config.tile_size) as i32;

    let grass_dark = Color::srgb(0.22, 0.45, 0.15);
    let grass_light = Color::srgb(0.28, 0.52, 0.18);

    for x in 0..tile_count_x {
        for y in 0..tile_count_y {
            let color = if (x + y) % 2 == 0 {
                grass_dark
            } else {
                grass_light
            };

            let world_x = x as f32 * config.tile_size + config.tile_size / 2.0;
            let world_y = y as f32 * config.tile_size + config.tile_size / 2.0;

            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(config.tile_size)),
                    ..default()
                },
                Transform::from_xyz(world_x, world_y, 0.0),
            ));
        }
    }
}

fn spawn_nest_entrance(mut commands: Commands, config: Res<SimConfig>) {
    let nest_pos = config.nest_position;
    let size = config.tile_size * 3.0;

    commands.spawn((
        Sprite {
            color: Color::srgb(0.35, 0.22, 0.1),
            custom_size: Some(Vec2::new(size, size)),
            ..default()
        },
        Transform::from_xyz(nest_pos.x, nest_pos.y, 1.0),
        NestEntrance { colony_id: 0 },
    ));
}

fn spawn_food_sources(mut commands: Commands, config: Res<SimConfig>) {
    let mut rng = rand::thread_rng();
    let nest = config.nest_position;
    let margin = 150.0;

    let food_configs = [
        (50.0, 18.0, Color::srgb(0.9, 0.7, 0.2)),  // large fruit
        (50.0, 18.0, Color::srgb(0.85, 0.6, 0.15)),
        (20.0, 12.0, Color::srgb(0.6, 0.3, 0.2)),   // dead insect
        (20.0, 12.0, Color::srgb(0.55, 0.35, 0.2)),
        (5.0, 6.0, Color::srgb(0.9, 0.85, 0.7)),    // crumbs
        (5.0, 6.0, Color::srgb(0.85, 0.8, 0.65)),
        (5.0, 6.0, Color::srgb(0.88, 0.82, 0.68)),
        (5.0, 6.0, Color::srgb(0.92, 0.87, 0.72)),
    ];

    for (amount, size, color) in &food_configs {
        let mut x;
        let mut y;
        loop {
            x = rng.gen_range(margin..config.world_width - margin);
            y = rng.gen_range(margin..config.world_height - margin);
            // Keep food away from nest center
            if Vec2::new(x, y).distance(nest) > 200.0 {
                break;
            }
        }

        commands.spawn((
            Sprite {
                color: *color,
                custom_size: Some(Vec2::splat(*size)),
                ..default()
            },
            Transform::from_xyz(x, y, 1.5),
            FoodSource {
                remaining: *amount,
                max: *amount,
            },
        ));
    }
}
