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

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::SimulationPlugin;
    use crate::resources::simulation::{SimClock, SimSpeed};

    #[test]
    fn sim_plugin_advances_tick_when_running() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, SimulationPlugin));
        app.insert_resource(ButtonInput::<KeyCode>::default());

        let before = app.world().resource::<SimClock>().tick;
        app.update();
        let after = app.world().resource::<SimClock>().tick;

        assert_eq!(after, before + 1);
    }

    #[test]
    fn sim_plugin_does_not_advance_tick_when_paused() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, SimulationPlugin));
        app.insert_resource(ButtonInput::<KeyCode>::default());

        {
            let mut clock = app.world_mut().resource_mut::<SimClock>();
            clock.speed = SimSpeed::Paused;
        }
        let before = app.world().resource::<SimClock>().tick;
        app.update();
        let after = app.world().resource::<SimClock>().tick;

        assert_eq!(after, before);
    }
}
