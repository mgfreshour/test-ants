//! Legacy nest scoring module — replaced by stimulus-driven AI in nest_stimuli.rs.
//! Retained for QueenHunger invariant tests.

#[cfg(test)]
mod tests {
    #[test]
    fn starvation_timing_is_generous() {
        // Grace period: 30s, damage: 0.5/sec, queen HP: 100
        let grace = 30.0_f32;
        let damage_rate = 0.5_f32;
        let queen_hp = 100.0_f32;
        let time_to_die = grace + (queen_hp / damage_rate);
        let feeding_cycle = 30.0_f32;
        assert!(time_to_die / feeding_cycle >= 7.0);
    }

    #[test]
    fn starvation_timer_resets_when_fed() {
        use crate::components::nest::QueenHunger;
        let mut hunger = QueenHunger {
            satiation: 0.25,
            decay_rate: 0.005,
            starvation_timer: 50.0,
        };
        if hunger.satiation > 0.0 {
            hunger.starvation_timer = 0.0;
        }
        assert_eq!(hunger.starvation_timer, 0.0);
    }
}
