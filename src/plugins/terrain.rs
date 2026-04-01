use bevy::prelude::*;

use crate::components::terrain::NestEntrance;
use crate::resources::simulation::SimConfig;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_terrain, spawn_nest_entrance));
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
