use bevy::prelude::*;

use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};
use crate::sim_core::clock::clock_tick_step;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimClock>()
            .init_resource::<SimConfig>()
            .add_systems(PreUpdate, sim_clock_tick)
            .add_systems(Update, toggle_sim_speed);
    }
}

fn sim_clock_tick(mut clock: ResMut<SimClock>, time: Res<Time>) {
    let (elapsed, tick) = clock_tick_step(
        clock.elapsed,
        clock.tick,
        time.delta_secs(),
        clock.speed.multiplier(),
        clock.speed == SimSpeed::Paused,
    );
    clock.elapsed = elapsed;
    clock.tick = tick;
}

fn toggle_sim_speed(input: Res<ButtonInput<KeyCode>>, mut clock: ResMut<SimClock>) {
    if input.just_pressed(KeyCode::Space) {
        if clock.speed == SimSpeed::Paused {
            clock.speed = SimSpeed::Normal;
        } else {
            clock.speed = SimSpeed::Paused;
        }
    }
    if input.just_pressed(KeyCode::Period) {
        if clock.speed != SimSpeed::Paused {
            clock.speed = clock.speed.cycle_next();
        }
    }
}
