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
            // Initial food sources and nest entrances are now loaded from LDtk entities.
            .add_systems(Update, random_food_drops);
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
