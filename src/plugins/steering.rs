use bevy::prelude::*;
use crate::components::ant::{Movement, SteeringTarget, SteeringWeights};
use crate::resources::simulation::{SimClock, SimSpeed};
use crate::sim_core::steering;

pub struct SteeringPlugin;

const SEPARATION_RADIUS: f32 = 8.0;
const WAYPOINT_THRESHOLD: f32 = 3.0;

impl Plugin for SteeringPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, apply_steering);
    }
}

/// Unified steering application: reads SteeringTarget, computes final direction
fn apply_steering(
    clock: Res<SimClock>,
    mut query: Query<(
        &Transform,
        &mut Movement,
        &mut SteeringTarget,
        &SteeringWeights,
    )>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    // First pass: collect all positions for separation queries
    let positions: Vec<Vec2> = query
        .iter()
        .map(|(tf, _, _, _)| tf.translation.truncate())
        .collect();

    // Second pass: apply steering
    for (transform, mut movement, mut target, weights) in &mut query {
        let pos = transform.translation.truncate();
        let current_dir = movement.direction;

        // Find nearby ants within separation radius
        let nearby: Vec<Vec2> = positions
            .iter()
            .filter(|&&other_pos| {
                let dist = pos.distance(other_pos);
                dist > 0.01 && dist < SEPARATION_RADIUS
            })
            .copied()
            .collect();

        match *target {
            SteeringTarget::None => {
                // No steering target set — keep existing direction (set by legacy systems)
            }
            SteeringTarget::Direction { target: target_dir } => {
                let output = steering::compute_direction_steering(
                    pos,
                    current_dir,
                    target_dir,
                    weights,
                    &nearby,
                    SEPARATION_RADIUS,
                );
                movement.direction = output.direction;
            }
            SteeringTarget::Path {
                ref waypoints,
                index,
            } => {
                let (output, next_idx) = steering::compute_waypoint_steering(
                    pos,
                    current_dir,
                    waypoints,
                    index,
                    WAYPOINT_THRESHOLD,
                    weights,
                    &nearby,
                    SEPARATION_RADIUS,
                );
                movement.direction = output.direction;

                // Update waypoint index if advanced
                if let Some(new_idx) = next_idx {
                    if let SteeringTarget::Path { ref mut index, .. } = *target {
                        *index = new_idx;
                    }
                }
            }
        }
    }
}
