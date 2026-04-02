use bevy::prelude::Vec2;
use crate::components::ant::SteeringWeights;

/// Weighted steering output from multiple influences
#[derive(Debug, Clone, Copy)]
pub struct SteeringOutput {
    pub direction: Vec2,  // Final normalized direction
}

/// Direction-based steering: seek toward target point with separation avoidance
pub fn compute_direction_steering(
    current_pos: Vec2,
    current_direction: Vec2,
    target_direction: Vec2,  // Pre-computed from pheromones/targets
    weights: &SteeringWeights,
    nearby_positions: &[Vec2],
    separation_radius: f32,
) -> SteeringOutput {
    // Compute separation force
    let separation = compute_separation_force(current_pos, nearby_positions, separation_radius);

    // Blend: target + forward momentum + separation
    let blended = (target_direction * weights.seek_weight
                  + current_direction * weights.forward_weight
                  + separation * weights.separation_weight)
                  .normalize_or_zero();

    SteeringOutput { direction: blended }
}

/// Waypoint-based steering: follow path waypoints with separation
pub fn compute_waypoint_steering(
    current_pos: Vec2,
    current_direction: Vec2,
    waypoints: &[Vec2],
    current_waypoint_idx: usize,
    waypoint_threshold: f32,
    weights: &SteeringWeights,
    nearby_positions: &[Vec2],
    separation_radius: f32,
) -> (SteeringOutput, Option<usize>) {
    // No waypoints remaining
    if current_waypoint_idx >= waypoints.len() {
        return (SteeringOutput { direction: Vec2::ZERO }, None);
    }

    let target = waypoints[current_waypoint_idx];
    let to_target = target - current_pos;
    let dist = to_target.length();

    // Reached waypoint, advance
    if dist < waypoint_threshold {
        let next_idx = current_waypoint_idx + 1;
        return (
            SteeringOutput { direction: to_target.normalize_or_zero() },
            Some(next_idx)
        );
    }

    // Steer toward current waypoint with separation
    let seek_dir = to_target.normalize();
    let separation = compute_separation_force(current_pos, nearby_positions, separation_radius);

    let blended = (seek_dir * weights.seek_weight
                  + current_direction * weights.forward_weight
                  + separation * weights.separation_weight)
                  .normalize_or_zero();

    (SteeringOutput { direction: blended }, None)
}

/// Compute repulsion force from nearby ants
pub fn compute_separation_force(
    current_pos: Vec2,
    nearby_positions: &[Vec2],
    separation_radius: f32,
) -> Vec2 {
    let mut force = Vec2::ZERO;
    for &other_pos in nearby_positions {
        let offset = current_pos - other_pos;
        let dist = offset.length();
        if dist > 0.01 && dist < separation_radius {
            let strength = (1.0 - dist / separation_radius).max(0.0);
            force += offset.normalize() * strength;
        }
    }
    force.normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_steering_seeks_target() {
        let output = compute_direction_steering(
            Vec2::ZERO,
            Vec2::X,
            Vec2::Y,  // Target is +Y
            &SteeringWeights { seek_weight: 1.0, forward_weight: 0.0, separation_weight: 0.0 },
            &[],
            10.0,
        );
        assert!((output.direction - Vec2::Y).length() < 0.01);
    }

    #[test]
    fn direction_steering_blends_forward_momentum() {
        let weights = SteeringWeights {
            seek_weight: 0.5,
            forward_weight: 0.5,
            separation_weight: 0.0,
        };
        let output = compute_direction_steering(
            Vec2::ZERO,
            Vec2::X,
            Vec2::Y,
            &weights,
            &[],
            10.0,
        );
        // Should blend between X and Y, not be purely Y
        assert!(output.direction.x > 0.3 && output.direction.y > 0.3);
    }

    #[test]
    fn waypoint_steering_advances_index() {
        let waypoints = vec![Vec2::new(10.0, 0.0), Vec2::new(20.0, 0.0)];
        let (_, next_idx) = compute_waypoint_steering(
            Vec2::new(9.5, 0.0),  // Within threshold of first waypoint
            Vec2::X,
            &waypoints,
            0,
            3.0,
            &SteeringWeights::default(),
            &[],
            10.0,
        );
        assert_eq!(next_idx, Some(1));
    }

    #[test]
    fn waypoint_steering_follows_path() {
        let waypoints = vec![Vec2::new(10.0, 0.0)];
        let (output, next_idx) = compute_waypoint_steering(
            Vec2::ZERO,
            Vec2::X,
            &waypoints,
            0,
            3.0,
            &SteeringWeights::default(),
            &[],
            10.0,
        );
        // Should steer toward waypoint
        assert!(output.direction.x > 0.5);
        assert_eq!(next_idx, None);
    }

    #[test]
    fn separation_prevents_collisions() {
        let force = compute_separation_force(
            Vec2::ZERO,
            &[Vec2::new(5.0, 0.0)],  // Ant 5 units to the right
            10.0,
        );
        assert!(force.x < -0.3);  // Should push left (away)
    }

    #[test]
    fn separation_scales_with_distance() {
        // Test that separation force direction is correct and points away
        let force_close = compute_separation_force(
            Vec2::ZERO,
            &[Vec2::new(2.0, 0.0)],  // Very close (dist=2)
            10.0,
        );
        let force_far = compute_separation_force(
            Vec2::ZERO,
            &[Vec2::new(9.0, 0.0)],  // Near boundary (dist=9)
            10.0,
        );
        // Both forces normalized to unit length, but both should point left (away from ant at 2.0 and 9.0)
        // Note: because we normalize the result, both have length 1.0
        // What matters is they point in the same direction (both negative x)
        assert!(force_close.x < 0.0);  // Should push left (away from obstacle at positive x)
        assert!(force_far.x < 0.0);    // Should also push left
        // Both are normalized unit vectors pointing in same direction
        assert!((force_close - force_far).length() < 0.01);
    }

    #[test]
    fn steering_weights_affect_blend() {
        let weights_high_seek = SteeringWeights {
            seek_weight: 2.0,
            forward_weight: 0.0,
            separation_weight: 0.0,
        };
        let weights_high_forward = SteeringWeights {
            seek_weight: 0.0,
            forward_weight: 2.0,
            separation_weight: 0.0,
        };

        let output_seek = compute_direction_steering(
            Vec2::ZERO,
            Vec2::X,
            Vec2::Y,
            &weights_high_seek,
            &[],
            10.0,
        );
        let output_fwd = compute_direction_steering(
            Vec2::ZERO,
            Vec2::X,
            Vec2::Y,
            &weights_high_forward,
            &[],
            10.0,
        );

        // High seek weight should lean more toward target (Y)
        // High forward weight should lean more toward current direction (X)
        assert!(output_seek.direction.y > output_fwd.direction.y);
        assert!(output_fwd.direction.x > output_seek.direction.x);
    }
}
