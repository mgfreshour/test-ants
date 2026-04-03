//! Stimulus-driven nest AI logic.
//!
//! Pure functions for computing stimulus strengths and response thresholds.
//! Replaces the old utility-scoring approach with a biologically-inspired
//! model where ants respond to local stimuli based on individual thresholds.

use crate::components::ant::AntJob;

/// Types of stimuli an ant can respond to while wandering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StimulusType {
    HungryLarva,
    UnrelocatedBrood,
    LooseFood,
    HungryQueen,
    DigFace,
}

/// Per-task response thresholds. Lower = responds more easily.
#[derive(Debug, Clone, Copy)]
pub struct ThresholdSet {
    pub feed_larva: f32,
    pub move_brood: f32,
    pub haul_food: f32,
    pub attend_queen: f32,
    pub dig: f32,
}

impl ThresholdSet {
    /// Get threshold for a given stimulus type.
    pub fn get(&self, stimulus: StimulusType) -> f32 {
        match stimulus {
            StimulusType::HungryLarva => self.feed_larva,
            StimulusType::UnrelocatedBrood => self.move_brood,
            StimulusType::LooseFood => self.haul_food,
            StimulusType::HungryQueen => self.attend_queen,
            StimulusType::DigFace => self.dig,
        }
    }

    /// Reduce threshold for a completed task type (task inertia).
    pub fn apply_inertia(&mut self, completed: StimulusType) {
        let val = match completed {
            StimulusType::HungryLarva => &mut self.feed_larva,
            StimulusType::UnrelocatedBrood => &mut self.move_brood,
            StimulusType::LooseFood => &mut self.haul_food,
            StimulusType::HungryQueen => &mut self.attend_queen,
            StimulusType::DigFace => &mut self.dig,
        };
        *val = (*val - INERTIA_REDUCTION).max(INERTIA_FLOOR);
    }
}

const INERTIA_REDUCTION: f32 = 0.05;
const INERTIA_FLOOR: f32 = 0.05;

/// Initialize thresholds based on ant job.
/// Nurses are sensitive to brood/queen care; Diggers to excavation.
/// All ants can respond to any stimulus, just at different sensitivities.
pub fn default_thresholds(job: AntJob) -> ThresholdSet {
    match job {
        AntJob::Nurse => ThresholdSet {
            feed_larva: 0.2,
            move_brood: 0.3,
            haul_food: 0.4,
            attend_queen: 0.2,
            dig: 0.7,
        },
        AntJob::Digger => ThresholdSet {
            feed_larva: 0.7,
            move_brood: 0.7,
            haul_food: 0.4,
            attend_queen: 0.6,
            dig: 0.2,
        },
        // Surface jobs / unassigned: high thresholds, mostly wander
        _ => ThresholdSet {
            feed_larva: 0.9,
            move_brood: 0.9,
            haul_food: 0.7,
            attend_queen: 0.9,
            dig: 0.9,
        },
    }
}

/// Should an ant respond to a stimulus? Compares strength against threshold
/// with a crowding penalty from nearby workers already on the same task.
pub fn should_respond(strength: f32, threshold: f32, workers_on_task: usize) -> bool {
    let crowding = workers_on_task as f32 * 0.15;
    let effective_threshold = (threshold + crowding).min(1.0);
    strength > effective_threshold
}

/// Stimulus strength for a hungry larva.
/// Stronger when closer and when brood-need pheromone is high.
pub fn larva_stimulus_strength(distance_cells: f32, brood_pheromone: f32) -> f32 {
    let proximity = 1.0 / (1.0 + distance_cells * 0.5);
    let pheromone_boost = 0.3 + brood_pheromone * 0.7;
    (proximity * pheromone_boost).clamp(0.0, 1.0)
}

/// Stimulus strength for a hungry queen.
/// Driven primarily by queen hunger level and local queen pheromone signal.
pub fn queen_stimulus_strength(queen_hunger: f32, queen_signal: f32) -> f32 {
    // queen_hunger: 0 = fully fed, 1 = starving
    let urgency = 0.3 + queen_hunger * 0.7;
    let signal_factor = 0.5 + queen_signal * 0.5;
    (urgency * signal_factor).clamp(0.0, 1.0)
}

/// Stimulus strength for loose food at entrance.
/// Stronger when closer.
pub fn food_stimulus_strength(distance_cells: f32) -> f32 {
    (1.0 / (1.0 + distance_cells * 0.4)).clamp(0.0, 1.0)
}

/// Stimulus strength for unrelocated brood.
/// Stronger when closer.
pub fn brood_stimulus_strength(distance_cells: f32) -> f32 {
    (1.0 / (1.0 + distance_cells * 0.5)).clamp(0.0, 1.0)
}

/// Stimulus strength for a diggable face.
/// Stronger when closer and when construction pheromone is high.
pub fn dig_stimulus_strength(distance_cells: f32, construction_pheromone: f32) -> f32 {
    let proximity = 1.0 / (1.0 + distance_cells * 0.4);
    let pheromone_boost = 0.5 + construction_pheromone * 0.5;
    (proximity * pheromone_boost).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nurse_has_low_brood_threshold() {
        let t = default_thresholds(AntJob::Nurse);
        assert!(t.feed_larva < 0.3);
        assert!(t.attend_queen < 0.3);
    }

    #[test]
    fn digger_has_low_dig_threshold() {
        let t = default_thresholds(AntJob::Digger);
        assert!(t.dig < 0.3);
        assert!(t.feed_larva > 0.5);
    }

    #[test]
    fn both_jobs_have_moderate_haul_threshold() {
        let nurse = default_thresholds(AntJob::Nurse);
        let digger = default_thresholds(AntJob::Digger);
        assert_eq!(nurse.haul_food, digger.haul_food);
        assert!(nurse.haul_food < 0.5);
    }

    #[test]
    fn should_respond_when_strong_stimulus() {
        assert!(should_respond(0.8, 0.3, 0));
    }

    #[test]
    fn should_not_respond_below_threshold() {
        assert!(!should_respond(0.2, 0.3, 0));
    }

    #[test]
    fn crowding_raises_effective_threshold() {
        // Without crowding: 0.35 > 0.3 => respond
        assert!(should_respond(0.35, 0.3, 0));
        // With 2 workers: effective = 0.3 + 0.3 = 0.6 => 0.35 < 0.6 => no
        assert!(!should_respond(0.35, 0.3, 2));
    }

    #[test]
    fn crowding_caps_at_one() {
        // Even with many workers, threshold can't exceed 1.0
        assert!(!should_respond(0.99, 0.3, 100));
    }

    #[test]
    fn larva_strength_decreases_with_distance() {
        let close = larva_stimulus_strength(1.0, 0.5);
        let far = larva_stimulus_strength(5.0, 0.5);
        assert!(close > far);
    }

    #[test]
    fn larva_strength_increases_with_pheromone() {
        let low = larva_stimulus_strength(2.0, 0.1);
        let high = larva_stimulus_strength(2.0, 0.9);
        assert!(high > low);
    }

    #[test]
    fn queen_strength_scales_with_hunger() {
        let fed = queen_stimulus_strength(0.0, 0.5);
        let starving = queen_stimulus_strength(1.0, 0.5);
        assert!(starving > fed);
    }

    #[test]
    fn queen_strength_scales_with_signal() {
        let no_signal = queen_stimulus_strength(0.5, 0.0);
        let strong_signal = queen_stimulus_strength(0.5, 1.0);
        assert!(strong_signal > no_signal);
    }

    #[test]
    fn starving_queen_exceeds_digger_threshold() {
        let digger = default_thresholds(AntJob::Digger);
        let strength = queen_stimulus_strength(1.0, 0.8);
        assert!(should_respond(strength, digger.attend_queen, 0));
    }

    #[test]
    fn food_strength_decreases_with_distance() {
        let close = food_stimulus_strength(0.0);
        let far = food_stimulus_strength(5.0);
        assert!(close > far);
    }

    #[test]
    fn dig_strength_decreases_with_distance() {
        let close = dig_stimulus_strength(1.0, 0.5);
        let far = dig_stimulus_strength(5.0, 0.5);
        assert!(close > far);
    }

    #[test]
    fn dig_bootstraps_without_construction_pheromone() {
        // Regression: diggers must respond to adjacent dig faces even with zero
        // construction pheromone, otherwise the dig feedback loop never starts.
        let digger = default_thresholds(AntJob::Digger);
        let strength = dig_stimulus_strength(1.0, 0.0);
        assert!(
            should_respond(strength, digger.dig, 0),
            "dig stimulus {strength} must exceed digger threshold {} at dist=1 with no pheromone",
            digger.dig,
        );
        // Must also work with 1 crowding worker (threshold + 0.15)
        assert!(
            should_respond(strength, digger.dig, 1),
            "dig stimulus {strength} must exceed crowded threshold {} at dist=1",
            digger.dig + 0.15,
        );
    }

    #[test]
    fn task_inertia_lowers_threshold() {
        let mut t = default_thresholds(AntJob::Nurse);
        let before = t.feed_larva;
        t.apply_inertia(StimulusType::HungryLarva);
        assert!(t.feed_larva < before);
    }

    #[test]
    fn task_inertia_does_not_go_below_floor() {
        let mut t = default_thresholds(AntJob::Nurse);
        for _ in 0..100 {
            t.apply_inertia(StimulusType::HungryLarva);
        }
        assert!(t.feed_larva >= INERTIA_FLOOR);
    }

    #[test]
    fn surface_jobs_have_high_thresholds() {
        let t = default_thresholds(AntJob::Forager);
        assert!(t.feed_larva >= 0.9);
        assert!(t.dig >= 0.9);
        assert!(t.attend_queen >= 0.9);
    }
}
