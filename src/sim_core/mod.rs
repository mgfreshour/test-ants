//! Pure simulation logic module.
//!
//! This module is the home for deterministic simulation code that can be unit
//! tested without Bevy ECS world setup, rendering components, or UI concerns.

pub mod ant_logic;
pub mod nest_scoring;
pub mod nest_transitions;

/// Scale frame delta seconds by a simulation speed multiplier.
///
/// This helper is intentionally pure and side-effect free.
pub fn scaled_dt(delta_secs: f32, speed_multiplier: f32) -> f32 {
    delta_secs * speed_multiplier
}

#[cfg(test)]
mod tests {
    use super::scaled_dt;

    #[test]
    fn scales_delta_time_by_multiplier() {
        assert_eq!(scaled_dt(0.5, 4.0), 2.0);
    }
}
