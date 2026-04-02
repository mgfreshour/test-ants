use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{Ant, Health};
use crate::resources::simulation::{SimClock, SimSpeed};
use crate::plugins::player::ToastQueue;

/// Environment hazards and dynamic events: rain, flooding, footsteps, lawnmower, pesticide, day/night.
pub struct EnvironmentPlugin;

/// Message to trigger a hazard event manually.
#[derive(Message)]
pub enum HazardEvent {
    TriggerRain,
    TriggerFootstep,
    TriggerLawnmower,
    TriggerPesticide,
}

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EnvironmentState::default())
            .add_systems(Update, (
                handle_manual_hazards,
                update_day_night_cycle,
                update_rain_state,
                update_hazard_events,
            ).chain());
    }
}

/// Global environment state tracking weather, time, and active hazards.
#[derive(Resource, Clone, Debug)]
pub struct EnvironmentState {
    /// 0.0 = midnight, 0.5 = noon, cycles every 3 minutes (180 seconds)
    pub time_of_day: f32,
    /// true if currently raining
    pub is_raining: bool,
    /// time until next rain event (seconds)
    pub rain_timer: f32,
    /// base evaporation multiplier (1.0 normal, 10.0 during rain)
    pub evaporation_multiplier: f32,
    /// water level in nest (0.0 = dry, 1.0 = flooded)
    pub flood_level: f32,
    /// active hazard zones (position, radius, damage_per_tick)
    pub active_hazards: Vec<HazardZone>,
}

#[derive(Clone, Debug)]
pub struct HazardZone {
    pub position: Vec2,
    pub radius: f32,
    pub damage_per_tick: f32,
    pub remaining_time: f32,
}

impl Default for EnvironmentState {
    fn default() -> Self {
        Self {
            time_of_day: 0.5, // Start at noon
            is_raining: false,
            rain_timer: 120.0, // First rain in 2 minutes
            evaporation_multiplier: 1.0,
            flood_level: 0.0,
            active_hazards: Vec::new(),
        }
    }
}

/// Update day/night cycle (3-minute cycle).
fn update_day_night_cycle(
    mut env: ResMut<EnvironmentState>,
    sim_clock: Res<SimClock>,
    time: Res<Time>,
) {
    if sim_clock.speed == SimSpeed::Paused {
        return;
    }

    let cycle_duration = 180.0; // 3 minutes
    let delta = time.delta_secs() * sim_clock.speed.multiplier();

    env.time_of_day = (env.time_of_day + delta / cycle_duration) % 1.0;
}

/// Rain event management and pheromone impact.
fn update_rain_state(
    mut env: ResMut<EnvironmentState>,
    sim_clock: Res<SimClock>,
    time: Res<Time>,
    mut toasts: ResMut<ToastQueue>,
) {
    if sim_clock.speed == SimSpeed::Paused {
        return;
    }

    let delta = time.delta_secs() * sim_clock.speed.multiplier();

    if !env.is_raining {
        env.rain_timer -= delta;
        if env.rain_timer <= 0.0 {
            // Start rain
            env.is_raining = true;
            env.evaporation_multiplier = 10.0;
            env.flood_level = 0.0;
            toasts.push("Rain starting!".to_string());
        }
    } else {
        // Rain active for 30-60 seconds
        env.flood_level += delta * 0.5; // Gradually fill
        if env.flood_level > 1.0 {
            env.flood_level = 1.0;
        }

        if env.rain_timer > 0.0 {
            env.rain_timer -= delta;
        } else {
            // Stop rain after 60 seconds
            env.is_raining = false;
            env.evaporation_multiplier = 1.0;
            env.rain_timer = rand::thread_rng().gen_range(120.0..300.0); // 2-5 min until next rain
        }
    }

    // Drain flood level gradually
    if !env.is_raining && env.flood_level > 0.0 {
        env.flood_level = (env.flood_level - delta * 0.1).max(0.0);
    }
}

/// Handle manually triggered hazard events from UI.
fn handle_manual_hazards(
    mut env: ResMut<EnvironmentState>,
    mut events: MessageReader<HazardEvent>,
    mut toasts: ResMut<ToastQueue>,
) {
    for event in events.read() {
        match event {
            HazardEvent::TriggerRain => {
                env.is_raining = true;
                env.evaporation_multiplier = 10.0;
                env.flood_level = 0.0;
                env.rain_timer = 60.0;
                toasts.push("Rain triggered!".to_string());
            }
            HazardEvent::TriggerFootstep => {
                let pos = Vec2::new(
                    rand::thread_rng().gen_range(50.0..1230.0),
                    rand::thread_rng().gen_range(50.0..670.0),
                );
                env.active_hazards.push(HazardZone {
                    position: pos,
                    radius: 30.0,
                    damage_per_tick: 999.0,
                    remaining_time: 0.5,
                });
                toasts.push("Footstep!".to_string());
            }
            HazardEvent::TriggerLawnmower => {
                let y = rand::thread_rng().gen_range(100.0..650.0);
                env.active_hazards.push(HazardZone {
                    position: Vec2::new(640.0, y),
                    radius: 60.0,
                    damage_per_tick: 999.0,
                    remaining_time: 3.0,
                });
                toasts.push("Lawnmower!".to_string());
            }
            HazardEvent::TriggerPesticide => {
                let pos = Vec2::new(
                    rand::thread_rng().gen_range(100.0..1180.0),
                    rand::thread_rng().gen_range(100.0..620.0),
                );
                env.active_hazards.push(HazardZone {
                    position: pos,
                    radius: 50.0,
                    damage_per_tick: 5.0,
                    remaining_time: 30.0,
                });
                toasts.push("Pesticide spray!".to_string());
            }
        }
    }
}

/// Manage hazard events: footsteps, lawnmower, pesticide.
fn update_hazard_events(
    mut env: ResMut<EnvironmentState>,
    sim_clock: Res<SimClock>,
    time: Res<Time>,
    mut toasts: ResMut<ToastQueue>,
    mut query: Query<(&Transform, &mut Health), With<Ant>>,
) {
    if sim_clock.speed == SimSpeed::Paused {
        return;
    }

    let delta = time.delta_secs() * sim_clock.speed.multiplier();

    // Update existing hazard zones
    env.active_hazards.retain_mut(|hazard| {
        hazard.remaining_time -= delta;
        hazard.remaining_time > 0.0
    });

    // Apply damage from active hazards
    for (transform, mut health) in query.iter_mut() {
        for hazard in &env.active_hazards {
            let dist = transform.translation.truncate().distance(hazard.position);
            if dist < hazard.radius {
                health.current -= hazard.damage_per_tick * delta;
            }
        }
    }

    // Random hazard events (roughly every 30-60 seconds)
    let mut rng = rand::thread_rng();
    if rng.gen::<f32>() < 0.001 {
        let event_type = rng.gen_range(0..3);
        match event_type {
            0 => {
                // Footstep
                let pos = Vec2::new(
                    rng.gen_range(50.0..1230.0),
                    rng.gen_range(50.0..670.0),
                );
                env.active_hazards.push(HazardZone {
                    position: pos,
                    radius: 30.0,
                    damage_per_tick: 999.0, // Instant death
                    remaining_time: 0.5, // Very brief
                });
                toasts.push("Watch out — footstep!".to_string());
            }
            1 => {
                // Lawnmower (horizontal sweep)
                let y = rng.gen_range(100.0..650.0);
                env.active_hazards.push(HazardZone {
                    position: Vec2::new(640.0, y),
                    radius: 60.0, // Wide swath
                    damage_per_tick: 999.0,
                    remaining_time: 3.0, // Sweeps across for 3 seconds
                });
                toasts.push("Lawnmower approaching!".to_string());
            }
            2 => {
                // Pesticide zone
                let pos = Vec2::new(
                    rng.gen_range(100.0..1180.0),
                    rng.gen_range(100.0..620.0),
                );
                env.active_hazards.push(HazardZone {
                    position: pos,
                    radius: 50.0,
                    damage_per_tick: 5.0, // Damage over time
                    remaining_time: 30.0,
                });
                toasts.push("Pesticide spray detected!".to_string());
            }
            _ => {}
        }
    }
}
