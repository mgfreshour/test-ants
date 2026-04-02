use bevy::prelude::*;
use rand::Rng;
use crate::components::ant::{Ant, AntJob, AntState, ColonyMember, Movement, PlayerControlled, SteeringTarget, SteeringWeights, TrailSense};
use crate::components::map::MapPortal;
use crate::resources::active_map::MapRegistry;
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};

const PATROL_RADIUS: f32 = 120.0;
const FORWARD_WEIGHT: f32 = 0.5;

/// Defender ants patrol near the nest entrance on the surface.
/// They wander within PATROL_RADIUS of the nearest portal, circling back
/// when they stray too far.
pub fn ant_defender_patrol(
    clock: Res<SimClock>,
    config: Res<SimConfig>,
    registry: Res<MapRegistry>,
    portal_query: Query<&MapPortal>,
    mut query: Query<
        (&Transform, &mut Movement, &Ant, &AntJob, &ColonyMember, &mut TrailSense, &mut SteeringTarget, &mut SteeringWeights),
        Without<PlayerControlled>,
    >,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let mut rng = rand::thread_rng();

    for (transform, mut movement, ant, job, colony, mut sense, mut steering_target, mut steering_weights) in &mut query {
        if *job != AntJob::Defender {
            continue;
        }
        // Defenders in combat states are handled by combat/recruiting systems
        if matches!(ant.state, AntState::Defending | AntState::Fighting | AntState::Following | AntState::Attacking) {
            continue;
        }

        let pos = transform.translation.truncate();
        let fwd = movement.direction;

        // Find the nearest portal on the surface for this colony
        let patrol_center = portal_query
            .iter()
            .filter(|p| {
                p.map == registry.surface
                    && p.colony_id.map_or(true, |id| id == colony.colony_id)
            })
            .min_by(|a, b| {
                pos.distance(a.position)
                    .partial_cmp(&pos.distance(b.position))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|p| p.position)
            .unwrap_or(config.nest_position);

        let to_center = patrol_center - pos;
        let dist = to_center.length();

        // If beyond patrol radius, steer back toward center
        let return_bias = if dist > PATROL_RADIUS {
            to_center.normalize_or_zero() * ((dist - PATROL_RADIUS) / PATROL_RADIUS).min(1.0)
        } else {
            Vec2::ZERO
        };

        // Random wandering with return bias
        let noise = config.exploration_noise * 0.8;
        let angle_offset = rng.gen_range(-noise..noise) * std::f32::consts::TAU;
        let current_angle = fwd.y.atan2(fwd.x);
        let new_angle = current_angle + angle_offset;
        let perturbed = Vec2::new(new_angle.cos(), new_angle.sin());

        let target_dir = (fwd * FORWARD_WEIGHT + perturbed * 0.5 + return_bias)
            .normalize_or_zero();

        *steering_target = SteeringTarget::Direction { target: target_dir };
        *steering_weights = SteeringWeights {
            seek_weight: 1.0,
            separation_weight: 0.5,
            forward_weight: FORWARD_WEIGHT,
        };
        *sense = TrailSense::Searching;
    }
}
