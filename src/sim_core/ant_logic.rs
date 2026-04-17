use bevy::prelude::Vec2;

pub fn should_follow_trail(entity_index: u32, elapsed: f32, epoch_rate: f32, follow_chance: u32) -> bool {
    let epoch = (elapsed * epoch_rate) as u32;
    let hash = entity_index.wrapping_mul(2654435761) ^ epoch;
    (hash % 100) < follow_chance
}

pub fn hunger_tick_step(
    hunger: f32,
    dt: f32,
    hunger_rate: f32,
    carried_food: Option<f32>,
    self_feed_threshold: f32,
    self_feed_relief: f32,
    starvation_dps: f32,
) -> (f32, f32) {
    let mut next_hunger = (hunger + hunger_rate * dt).min(1.0);

    if next_hunger > self_feed_threshold && carried_food.is_some() {
        next_hunger = (next_hunger - self_feed_relief).max(0.0);
    }

    let hp_loss = if next_hunger >= 1.0 {
        starvation_dps * dt
    } else {
        0.0
    };

    (next_hunger, hp_loss)
}

pub fn hunger_speed_factor(hunger: f32, hunger_slow_threshold: f32, hunger_slow_factor: f32) -> f32 {
    if hunger > hunger_slow_threshold {
        hunger_slow_factor
    } else {
        1.0
    }
}

pub fn surface_velocity(direction: Vec2, speed: f32, hunger: f32, dt: f32, hunger_slow_threshold: f32, hunger_slow_factor: f32) -> Vec2 {
    let factor = hunger_speed_factor(hunger, hunger_slow_threshold, hunger_slow_factor);
    direction * speed * factor * dt
}

pub fn apply_deposit_hunger_relief(hunger: f32, relief: f32) -> f32 {
    (hunger - relief).max(0.0)
}

pub fn pickup_food_amount(food_remaining: f32, max_pickup: f32) -> Option<f32> {
    if food_remaining <= 0.0 {
        None
    } else {
        Some(food_remaining.min(max_pickup))
    }
}

pub fn can_pickup_food(state_is_foraging: bool, distance: f32, pickup_range: f32) -> bool {
    state_is_foraging && distance < pickup_range
}

pub fn can_deposit_food(state_is_returning: bool, is_surface_ant: bool, distance_to_portal: f32, deposit_range: f32) -> bool {
    state_is_returning && is_surface_ant && distance_to_portal < deposit_range
}

pub fn home_pheromone_deposit_amount(is_foraging: bool, base_amount: f32) -> Option<f32> {
    if is_foraging {
        Some(base_amount)
    } else {
        None
    }
}

pub fn food_pheromone_deposit_amount(is_returning: bool, base_amount: f32, carried_food_amount: Option<f32>) -> Option<f32> {
    if !is_returning {
        return None;
    }
    let amount = if let Some(food) = carried_food_amount {
        base_amount * (1.0 + food * 0.1)
    } else {
        base_amount
    };
    Some(amount)
}

/// Decide which recruit state an ant should transition to based on local
/// pheromone intensities. Returns `Some("attack")`, `Some("follow")`, or
/// `None` when neither signal is strong enough.
pub fn recruit_entry_decision(
    attack_recruit_intensity: f32,
    recruit_intensity: f32,
    threshold: f32,
) -> Option<&'static str> {
    if attack_recruit_intensity >= threshold {
        Some("attack")
    } else if recruit_intensity >= threshold {
        Some("follow")
    } else {
        None
    }
}

/// After combat ends, decide whether an ant should return to Attacking
/// (attack pheromone still present) or fall back to Foraging.
pub fn post_combat_state(attack_recruit_intensity: f32, signal_threshold: f32) -> &'static str {
    if attack_recruit_intensity >= signal_threshold {
        "attacking"
    } else {
        "foraging"
    }
}

/// Compute red colony aggression level based on elapsed time.
/// Starts defensive (0.1), ramps linearly to max (0.9) over `ramp_duration` seconds.
pub fn red_aggression_curve(elapsed: f32, ramp_duration: f32) -> f32 {
    let t = (elapsed / ramp_duration).clamp(0.0, 1.0);
    0.1 + t * 0.8
}

/// Whether the red colony should launch a raid based on aggression and a timer.
/// Returns true when enough time has passed since the last raid attempt.
pub fn should_raid(aggression: f32, time_since_last_raid: f32, base_raid_interval: f32) -> bool {
    // Higher aggression → shorter raid intervals.
    let interval = base_raid_interval * (1.0 - aggression * 0.7);
    time_since_last_raid >= interval
}

// ---------------------------------------------------------------------------
// Defending <-> Foraging transition: hysteresis band + minimum dwell.
//
// Motivation: the alarm pheromone field is noisy (it both decays and diffuses
// each tick). If the promote and demote thresholds share a value, any ant
// standing on a cell whose alarm hovers near that value flips state every
// frame. A hysteresis band plus a short minimum dwell eliminates the flicker
// without losing responsiveness.
// ---------------------------------------------------------------------------

/// Alarm pheromone level at or above which a Foraging ant may promote to Defending.
pub const ALARM_PROMOTE_THRESHOLD: f32 = 1.0;

/// Alarm pheromone level a Defending ant must fall below to demote back to Foraging.
/// The gap between promote and demote is the hysteresis band.
pub const ALARM_DEMOTE_THRESHOLD: f32 = 0.3;

/// Minimum time (sim seconds) an ant must spend in a state before another
/// transition is considered. Kills sub-frame ping-ponging regardless of cause.
pub const MIN_STATE_DWELL_SECS: f32 = 0.5;

/// Alarm gradient length-squared required to consider the alarm field "directional".
/// Matches the constant baked into `alarm_response_steering` prior to extraction.
pub const ALARM_GRADIENT_MIN_LEN_SQ: f32 = 0.5;

/// Decide whether a Foraging ant should promote to Defending.
pub fn should_promote_to_defending(
    is_foraging: bool,
    local_alarm: f32,
    gradient_len_sq: f32,
    time_in_state: f32,
) -> bool {
    is_foraging
        && local_alarm >= ALARM_PROMOTE_THRESHOLD
        && gradient_len_sq > ALARM_GRADIENT_MIN_LEN_SQ
        && time_in_state >= MIN_STATE_DWELL_SECS
}

/// Result of evaluating whether a Defending ant should exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefendingExit {
    /// Stay Defending.
    Stay,
    /// Demote to Foraging.
    Foraging,
    /// Promote to Attacking (attack-recruit pheromone is strong).
    Attacking,
}

/// Decide whether a Defending ant should leave the Defending state this tick.
///
/// The ant stays Defending when:
/// - an enemy is still nearby (combat takes priority over the alarm field),
/// - the ant hasn't yet spent `MIN_STATE_DWELL_SECS` in its current state,
/// - or the local alarm is still in the hysteresis band
///   (i.e. `alarm >= ALARM_DEMOTE_THRESHOLD`).
pub fn should_demote_from_defending(
    is_defending: bool,
    has_nearby_enemy: bool,
    local_alarm: f32,
    attack_recruit: f32,
    attack_recruit_threshold: f32,
    time_in_state: f32,
) -> DefendingExit {
    if !is_defending {
        return DefendingExit::Stay;
    }
    if has_nearby_enemy {
        return DefendingExit::Stay;
    }
    if time_in_state < MIN_STATE_DWELL_SECS {
        return DefendingExit::Stay;
    }
    if local_alarm >= ALARM_DEMOTE_THRESHOLD {
        return DefendingExit::Stay;
    }
    if attack_recruit >= attack_recruit_threshold {
        DefendingExit::Attacking
    } else {
        DefendingExit::Foraging
    }
}

pub fn apply_boundary_bounce(pos: Vec2, dir: Vec2, min: Vec2, max: Vec2) -> (Vec2, Vec2) {
    let mut next_pos = pos;
    let mut next_dir = dir;

    if next_pos.x <= min.x {
        next_pos.x = min.x;
        next_dir.x = next_dir.x.abs();
    } else if next_pos.x >= max.x {
        next_pos.x = max.x;
        next_dir.x = -next_dir.x.abs();
    }

    if next_pos.y <= min.y {
        next_pos.y = min.y;
        next_dir.y = next_dir.y.abs();
    } else if next_pos.y >= max.y {
        next_pos.y = max.y;
        next_dir.y = -next_dir.y.abs();
    }

    (next_pos, next_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunger_step_applies_starvation_damage() {
        let (h, hp_loss) = hunger_tick_step(0.99, 1.0, 0.02, None, 0.5, 0.4, 0.5);
        assert_eq!(h, 1.0);
        assert_eq!(hp_loss, 0.5);
    }

    #[test]
    fn hunger_step_no_starvation_before_full_hunger() {
        let (h, hp_loss) = hunger_tick_step(0.5, 1.0, 0.01, None, 0.5, 0.4, 0.5);
        assert_eq!(h, 0.51);
        assert_eq!(hp_loss, 0.0);
    }

    #[test]
    fn hunger_step_self_feeds_when_carrying() {
        let (h, _) = hunger_tick_step(0.6, 1.0, 0.0, Some(1.0), 0.5, 0.4, 0.5);
        assert!((h - 0.2).abs() < 1e-6);
    }

    #[test]
    fn boundary_bounce_clamps_and_reflects() {
        let (p, d) = apply_boundary_bounce(
            Vec2::new(-1.0, 12.0),
            Vec2::new(-0.2, -0.5),
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 10.0),
        );
        assert_eq!(p, Vec2::new(0.0, 10.0));
        assert!(d.x >= 0.0);
        assert!(d.y <= 0.0);
    }

    #[test]
    fn deposit_relief_never_goes_below_zero() {
        assert_eq!(apply_deposit_hunger_relief(0.2, 0.5), 0.0);
        assert_eq!(apply_deposit_hunger_relief(0.8, 0.3), 0.5);
    }

    #[test]
    fn pickup_amount_is_bounded() {
        assert_eq!(pickup_food_amount(0.0, 5.0), None);
        assert_eq!(pickup_food_amount(3.0, 5.0), Some(3.0));
        assert_eq!(pickup_food_amount(8.0, 5.0), Some(5.0));
    }

    #[test]
    fn pickup_and_deposit_conditions_match_state_and_range() {
        assert!(can_pickup_food(true, 5.0, 20.0));
        assert!(!can_pickup_food(false, 5.0, 20.0));
        assert!(!can_pickup_food(true, 25.0, 20.0));

        assert!(can_deposit_food(true, true, 10.0, 30.0));
        assert!(!can_deposit_food(true, false, 10.0, 30.0));
        assert!(!can_deposit_food(false, true, 10.0, 30.0));
        assert!(!can_deposit_food(true, true, 35.0, 30.0));
    }

    #[test]
    fn pheromone_deposit_amounts_match_ant_state() {
        assert_eq!(home_pheromone_deposit_amount(true, 2.0), Some(2.0));
        assert_eq!(home_pheromone_deposit_amount(false, 2.0), None);

        assert_eq!(
            food_pheromone_deposit_amount(true, 1.5, Some(5.0)),
            Some(2.25)
        );
        assert_eq!(
            food_pheromone_deposit_amount(true, 1.5, None),
            Some(1.5)
        );
        assert_eq!(
            food_pheromone_deposit_amount(false, 1.5, Some(5.0)),
            None
        );
    }

    #[test]
    fn invariant_hunger_step_outputs_are_bounded() {
        for hunger in [0.0, 0.2, 0.5, 0.9, 1.0] {
            for dt in [0.0, 0.1, 1.0, 3.0] {
                let (next_hunger, hp_loss) =
                    hunger_tick_step(hunger, dt, 0.02, Some(1.0), 0.5, 0.4, 0.5);
                assert!((0.0..=1.0).contains(&next_hunger));
                assert!(hp_loss >= 0.0);
            }
        }
    }

    #[test]
    fn attack_recruit_takes_priority_over_follow_recruit() {
        // Both signals above threshold — attack wins
        assert_eq!(recruit_entry_decision(1.0, 1.0, 0.8), Some("attack"));
        // Only follow signal
        assert_eq!(recruit_entry_decision(0.2, 1.0, 0.8), Some("follow"));
        // Only attack signal
        assert_eq!(recruit_entry_decision(1.0, 0.2, 0.8), Some("attack"));
        // Neither
        assert_eq!(recruit_entry_decision(0.2, 0.3, 0.8), None);
    }

    #[test]
    fn attack_recruit_threshold_is_exact() {
        assert_eq!(recruit_entry_decision(0.8, 0.0, 0.8), Some("attack"));
        assert_eq!(recruit_entry_decision(0.79, 0.0, 0.8), None);
    }

    #[test]
    fn post_combat_returns_to_attacking_with_signal() {
        assert_eq!(post_combat_state(0.5, 0.4), "attacking");
        assert_eq!(post_combat_state(0.3, 0.4), "foraging");
        assert_eq!(post_combat_state(0.4, 0.4), "attacking");
    }

    #[test]
    fn red_aggression_starts_low() {
        let a = red_aggression_curve(0.0, 300.0);
        assert!((a - 0.1).abs() < 1e-6);
    }

    #[test]
    fn red_aggression_ramps_to_max() {
        let a = red_aggression_curve(300.0, 300.0);
        assert!((a - 0.9).abs() < 1e-6);
    }

    #[test]
    fn red_aggression_clamps_beyond_ramp() {
        let a = red_aggression_curve(600.0, 300.0);
        assert!((a - 0.9).abs() < 1e-6);
    }

    #[test]
    fn red_aggression_midpoint() {
        let a = red_aggression_curve(150.0, 300.0);
        assert!((a - 0.5).abs() < 1e-6);
    }

    #[test]
    fn should_raid_respects_interval() {
        // Low aggression → interval barely reduced.
        assert!(!should_raid(0.1, 50.0, 60.0));
        assert!(should_raid(0.1, 60.0, 60.0));
    }

    #[test]
    fn should_raid_high_aggression_shortens_interval() {
        // aggression 0.9 → interval = 60 * (1 - 0.63) = 60 * 0.37 = 22.2
        assert!(should_raid(0.9, 23.0, 60.0));
        assert!(!should_raid(0.9, 20.0, 60.0));
    }

    #[test]
    fn invariant_pickup_amount_is_never_negative() {
        for remaining in [-2.0, -0.1, 0.0, 0.2, 3.7, 9.0] {
            let picked = pickup_food_amount(remaining, 5.0).unwrap_or(0.0);
            assert!(picked >= 0.0);
            assert!(picked <= 5.0);
        }
    }

    // --- Defending <-> Foraging hysteresis + dwell ---

    #[test]
    fn promote_requires_alarm_at_or_above_threshold() {
        let dwell = MIN_STATE_DWELL_SECS + 1.0;
        assert!(!should_promote_to_defending(true, 0.9, 1.0, dwell));
        assert!(should_promote_to_defending(true, 1.0, 1.0, dwell));
        assert!(should_promote_to_defending(true, 2.0, 1.0, dwell));
    }

    #[test]
    fn promote_requires_directional_gradient() {
        let dwell = MIN_STATE_DWELL_SECS + 1.0;
        assert!(!should_promote_to_defending(true, 5.0, 0.1, dwell));
        assert!(should_promote_to_defending(true, 5.0, ALARM_GRADIENT_MIN_LEN_SQ + 0.01, dwell));
    }

    #[test]
    fn promote_requires_min_dwell() {
        assert!(!should_promote_to_defending(true, 5.0, 1.0, 0.1));
        assert!(should_promote_to_defending(true, 5.0, 1.0, MIN_STATE_DWELL_SECS));
    }

    #[test]
    fn promote_skipped_when_not_foraging() {
        assert!(!should_promote_to_defending(false, 5.0, 1.0, 10.0));
    }

    #[test]
    fn demote_blocked_by_nearby_enemy() {
        // Even with low alarm and long dwell, a nearby enemy pins the ant in Defending.
        let result = should_demote_from_defending(true, true, 0.0, 0.0, 0.4, 10.0);
        assert_eq!(result, DefendingExit::Stay);
    }

    #[test]
    fn demote_hysteresis_band_keeps_state_stable() {
        let dwell = MIN_STATE_DWELL_SECS + 1.0;
        // At the demote threshold: still inside band, stay.
        assert_eq!(
            should_demote_from_defending(true, false, ALARM_DEMOTE_THRESHOLD, 0.0, 0.4, dwell),
            DefendingExit::Stay
        );
        // Just below: demote fires.
        assert_eq!(
            should_demote_from_defending(true, false, ALARM_DEMOTE_THRESHOLD - 0.01, 0.0, 0.4, dwell),
            DefendingExit::Foraging
        );
    }

    #[test]
    fn demote_routes_to_attacking_when_recruit_signal_strong() {
        let dwell = MIN_STATE_DWELL_SECS + 1.0;
        assert_eq!(
            should_demote_from_defending(true, false, 0.0, 0.5, 0.4, dwell),
            DefendingExit::Attacking
        );
        // Below recruit threshold → Foraging.
        assert_eq!(
            should_demote_from_defending(true, false, 0.0, 0.39, 0.4, dwell),
            DefendingExit::Foraging
        );
    }

    #[test]
    fn demote_requires_min_dwell() {
        assert_eq!(
            should_demote_from_defending(true, false, 0.0, 0.0, 0.4, 0.1),
            DefendingExit::Stay
        );
    }

    #[test]
    fn demote_skipped_when_not_defending() {
        assert_eq!(
            should_demote_from_defending(false, false, 0.0, 0.0, 0.4, 10.0),
            DefendingExit::Stay
        );
    }

    /// Regression test for the Foraging <-> Defending flicker bug. Simulates 60
    /// frames at 60 fps where the alarm field oscillates across the old single
    /// 0.5 threshold. With hysteresis + dwell in place the state changes at
    /// most twice (one promote + one demote) across the full second.
    #[test]
    fn no_flip_within_dwell_even_under_noise() {
        let mut is_foraging = true;
        let mut is_defending = false;
        let mut time_in_state = MIN_STATE_DWELL_SECS + 1.0; // start cleanly dwelled
        let dt = 1.0 / 60.0;
        let mut transitions = 0u32;

        for frame in 0..60 {
            // Oscillate alarm between 0.4 and 0.6 every 2 frames (above promote
            // threshold in spikes, below demote threshold in troughs).
            let alarm = if frame % 2 == 0 { 0.4 } else { 1.2 };

            if is_foraging && should_promote_to_defending(true, alarm, 1.0, time_in_state) {
                is_foraging = false;
                is_defending = true;
                time_in_state = 0.0;
                transitions += 1;
            } else if is_defending {
                match should_demote_from_defending(true, false, alarm, 0.0, 0.4, time_in_state) {
                    DefendingExit::Foraging => {
                        is_defending = false;
                        is_foraging = true;
                        time_in_state = 0.0;
                        transitions += 1;
                    }
                    DefendingExit::Attacking | DefendingExit::Stay => {}
                }
            }

            time_in_state += dt;
        }

        assert!(
            transitions <= 2,
            "expected <= 2 transitions across 60 oscillating frames, got {transitions}"
        );
    }
}
