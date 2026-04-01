use bevy::prelude::*;
use rand::Rng;

use crate::components::map::MapId;
use crate::components::terrain::FoodSource;
use crate::resources::active_map::MapRegistry;
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};

pub struct TerrainPlugin;

/// Average interval between random food drops (in sim-seconds).
const FOOD_DROP_INTERVAL: f32 = 60.0;

#[derive(Resource)]
struct FoodDropTimer(f32);

impl Default for FoodDropTimer {
    fn default() -> Self {
        Self(FOOD_DROP_INTERVAL)
    }
}

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FoodDropTimer>()
            .add_systems(Startup, (setup_terrain, spawn_food_sources))
            .add_systems(Update, random_food_drops);
    }
}

fn setup_terrain(mut commands: Commands, config: Res<SimConfig>, registry: Res<MapRegistry>) {
    let tile_count_x = (config.world_width / config.tile_size) as i32;
    let tile_count_y = (config.world_height / config.tile_size) as i32;

    let grass_dark = Color::srgb(0.22, 0.45, 0.15);
    let grass_light = Color::srgb(0.28, 0.52, 0.18);

    // Nest entrance marker — dark mound with hole
    let np = config.nest_position;
    let mound_color = Color::srgb(0.35, 0.25, 0.15);
    let hole_color = Color::srgb(0.08, 0.05, 0.02);
    // Outer mound ring
    commands.spawn((
        Sprite {
            color: mound_color,
            custom_size: Some(Vec2::splat(28.0)),
            ..default()
        },
        Transform::from_xyz(np.x, np.y, 1.0),
        MapId(registry.surface),
    ));
    // Inner dark hole
    commands.spawn((
        Sprite {
            color: hole_color,
            custom_size: Some(Vec2::splat(14.0)),
            ..default()
        },
        Transform::from_xyz(np.x, np.y, 1.1),
        MapId(registry.surface),
    ));

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
                MapId(registry.surface),
            ));
        }
    }
}

fn spawn_food_sources(mut commands: Commands, config: Res<SimConfig>, registry: Res<MapRegistry>) {
    let mut rng = rand::thread_rng();
    let nest = config.nest_position;
    let margin = 150.0;

    let food_configs = [
        (15.0, 18.0, Color::srgb(0.9, 0.7, 0.2)),  // large fruit
        (15.0, 18.0, Color::srgb(0.85, 0.6, 0.15)),
        (15.0, 18.0, Color::srgb(0.85, 0.6, 0.15)),
        (15.0, 18.0, Color::srgb(0.85, 0.6, 0.15)),
        (15.0, 18.0, Color::srgb(0.85, 0.6, 0.15)),
        (15.0, 18.0, Color::srgb(0.85, 0.6, 0.15)),
        (15.0, 18.0, Color::srgb(0.85, 0.6, 0.15)),
        (12.0, 12.0, Color::srgb(0.6, 0.3, 0.2)),   // dead insect
        (12.0, 12.0, Color::srgb(0.55, 0.35, 0.2)),
        (12.0, 12.0, Color::srgb(0.55, 0.35, 0.2)),
        (12.0, 12.0, Color::srgb(0.55, 0.35, 0.2)),
        (12.0, 12.0, Color::srgb(0.55, 0.35, 0.2)),
        (12.0, 12.0, Color::srgb(0.55, 0.35, 0.2)),
        (5.0, 6.0, Color::srgb(0.9, 0.85, 0.7)),    // crumbs
        (5.0, 6.0, Color::srgb(0.85, 0.8, 0.65)),
        (5.0, 6.0, Color::srgb(0.88, 0.82, 0.68)),
        (5.0, 6.0, Color::srgb(0.92, 0.87, 0.72)),
        (5.0, 6.0, Color::srgb(0.92, 0.87, 0.72)),
        (5.0, 6.0, Color::srgb(0.92, 0.87, 0.72)),
        (5.0, 6.0, Color::srgb(0.92, 0.87, 0.72)),
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
            MapId(registry.surface),
        ));
    }
}

/// Periodically spawn a random food source on the surface.
fn random_food_drops(
    clock: Res<SimClock>,
    time: Res<Time>,
    config: Res<SimConfig>,
    registry: Res<MapRegistry>,
    mut timer: ResMut<FoodDropTimer>,
    mut commands: Commands,
    food_query: Query<&FoodSource>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    timer.0 -= dt;
    if timer.0 > 0.0 {
        return;
    }

    // Reset timer with some randomness (±30%)
    let mut rng = rand::thread_rng();
    timer.0 = FOOD_DROP_INTERVAL * rng.gen_range(0.7..1.3);

    // Cap total food sources on the map to avoid unbounded growth
    if food_query.iter().count() >= 20 {
        return;
    }

    let margin = 100.0;
    let x = rng.gen_range(margin..config.world_width - margin);
    let y = rng.gen_range(margin..config.world_height - margin);

    // Random size: small crumbs to medium piles
    let amount = rng.gen_range(15.0..80.0);
    let size = if amount > 50.0 { 12.0 } else { 6.0 };
    let green_tint = rng.gen_range(0.5..0.9);
    let color = Color::srgb(green_tint + 0.1, green_tint, rng.gen_range(0.1..0.3));

    commands.spawn((
        Sprite {
            color,
            custom_size: Some(Vec2::splat(size)),
            ..default()
        },
        Transform::from_xyz(x, y, 1.5),
        FoodSource {
            remaining: amount,
            max: amount,
        },
        MapId(registry.surface),
    ));
}
