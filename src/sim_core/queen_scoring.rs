//! Pure queen utility-scoring logic.
//!
//! All queen decision-making lives here as pure functions with no ECS
//! dependencies, making them trivially unit-testable.

/// Satiation consumed per egg.
pub const EGG_SATIATION_COST: f32 = 0.2;

/// Input snapshot for queen scoring — gathered from ECS in the plugin layer.
#[derive(Debug, Clone, Copy)]
pub struct QueenScoringInput {
    /// 0.0 = starving, 1.0 = fully fed.
    pub satiation: f32,
    /// Current health as fraction of max (0.0–1.0).
    pub health_frac: f32,
    /// Number of living brood (eggs + larvae + pupae).
    pub brood_count: u32,
    /// Total food stored in the colony.
    pub colony_food_stored: f32,
    /// Whether a queen chamber exists in the nest grid.
    pub has_queen_chamber: bool,
}

/// Raw utility scores for each queen task.
#[derive(Debug, Clone, Copy)]
pub struct QueenTaskScores {
    pub lay_eggs: f32,
    pub rest: f32,
    pub groom: f32,
    pub idle: f32,
}

/// The chosen task after scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueenTaskChoice {
    LayingEggs,
    Resting,
    Grooming,
    Idle,
}

/// Compute raw utility scores for each queen task.
pub fn compute_queen_scores(input: &QueenScoringInput) -> QueenTaskScores {
    // ── LayingEggs ─────────────────────────────────────────────
    let lay_eggs = if !input.has_queen_chamber || input.satiation < EGG_SATIATION_COST {
        0.0
    } else {
        // Base desire to lay scales with satiation
        let satiation_factor = (input.satiation - EGG_SATIATION_COST) / (1.0 - EGG_SATIATION_COST);
        // Diminishing returns as brood count grows
        let brood_penalty = 1.0 / (1.0 + input.brood_count as f32 * 0.05);
        0.8 * satiation_factor * brood_penalty
    };

    // ── Resting ────────────────────────────────────────────────
    let hunger_rest = if input.satiation < 0.3 {
        0.7 * (1.0 - input.satiation / 0.3)
    } else {
        0.0
    };
    let health_rest = if input.health_frac < 0.4 {
        0.7 * (1.0 - input.health_frac / 0.4)
    } else {
        0.0
    };
    let rest = hunger_rest.max(health_rest);

    // ── Grooming ───────────────────────────────────────────────
    let groom = 0.3; // constant moderate baseline

    // ── Idle ───────────────────────────────────────────────────
    let idle = 0.1; // low fallback

    QueenTaskScores { lay_eggs, rest, groom, idle }
}

/// Pick the highest-scoring task.
pub fn choose_queen_task(scores: &QueenTaskScores) -> QueenTaskChoice {
    let mut best = (QueenTaskChoice::Idle, scores.idle);

    if scores.groom > best.1 {
        best = (QueenTaskChoice::Grooming, scores.groom);
    }
    if scores.rest > best.1 {
        best = (QueenTaskChoice::Resting, scores.rest);
    }
    if scores.lay_eggs > best.1 {
        best = (QueenTaskChoice::LayingEggs, scores.lay_eggs);
    }

    best.0
}

/// Hunger decay multiplier when the queen is resting.
pub fn resting_decay_multiplier() -> f32 {
    0.5
}

/// Baseline signal emitted even when queen is fully fed, so workers can
/// still locate the queen for non-hunger tasks. Must be low enough that
/// it doesn't trigger attend-queen stimulus on its own.
const QUEEN_SIGNAL_BASELINE: f32 = 0.15;

/// Compute the queen signal strength to emit at the queen's position.
/// `hunger_frac`: 0.0 = fully fed, 1.0 = starving.
/// `max_strength`: config-driven cap (typically 1.0).
pub fn queen_hunger_signal(hunger_frac: f32, max_strength: f32) -> f32 {
    let scaled = QUEEN_SIGNAL_BASELINE + (1.0 - QUEEN_SIGNAL_BASELINE) * hunger_frac;
    (scaled * max_strength).clamp(0.0, max_strength)
}

// ── Builder for tests ──────────────────────────────────────────────

impl QueenScoringInput {
    /// Fully-fed, healthy queen with a queen chamber and no brood.
    pub fn default_test() -> Self {
        Self {
            satiation: 1.0,
            health_frac: 1.0,
            brood_count: 0,
            colony_food_stored: 100.0,
            has_queen_chamber: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laying_wins_when_well_fed_low_brood() {
        let input = QueenScoringInput::default_test();
        let choice = choose_queen_task(&compute_queen_scores(&input));
        assert_eq!(choice, QueenTaskChoice::LayingEggs);
    }

    #[test]
    fn laying_loses_when_satiation_below_egg_cost() {
        let input = QueenScoringInput {
            satiation: 0.15, // below EGG_SATIATION_COST
            ..QueenScoringInput::default_test()
        };
        let scores = compute_queen_scores(&input);
        assert_eq!(scores.lay_eggs, 0.0);
        assert_ne!(choose_queen_task(&scores), QueenTaskChoice::LayingEggs);
    }

    #[test]
    fn laying_diminishes_as_brood_count_rises() {
        let low_brood = QueenScoringInput { brood_count: 0, ..QueenScoringInput::default_test() };
        let high_brood = QueenScoringInput { brood_count: 40, ..QueenScoringInput::default_test() };
        let low_score = compute_queen_scores(&low_brood).lay_eggs;
        let high_score = compute_queen_scores(&high_brood).lay_eggs;
        assert!(low_score > high_score, "low brood {low_score} should beat high brood {high_score}");
    }

    #[test]
    fn resting_wins_when_satiation_critically_low() {
        let input = QueenScoringInput {
            satiation: 0.05,
            ..QueenScoringInput::default_test()
        };
        let choice = choose_queen_task(&compute_queen_scores(&input));
        assert_eq!(choice, QueenTaskChoice::Resting);
    }

    #[test]
    fn resting_wins_when_health_critically_low() {
        let input = QueenScoringInput {
            health_frac: 0.1,
            satiation: 0.5, // enough to lay, but health should override
            ..QueenScoringInput::default_test()
        };
        let choice = choose_queen_task(&compute_queen_scores(&input));
        assert_eq!(choice, QueenTaskChoice::Resting);
    }

    #[test]
    fn grooming_wins_as_default_when_nothing_scores_strongly() {
        let input = QueenScoringInput {
            satiation: 0.35, // above rest threshold, but lay_eggs score is weak
            health_frac: 1.0, // no need to rest
            brood_count: 30,  // high brood count suppresses laying further
            ..QueenScoringInput::default_test()
        };
        let choice = choose_queen_task(&compute_queen_scores(&input));
        assert_eq!(choice, QueenTaskChoice::Grooming);
    }

    #[test]
    fn grooming_beats_idle() {
        let scores = compute_queen_scores(&QueenScoringInput::default_test());
        assert!(scores.groom > scores.idle);
    }

    #[test]
    fn laying_zero_without_queen_chamber() {
        let input = QueenScoringInput {
            has_queen_chamber: false,
            ..QueenScoringInput::default_test()
        };
        let scores = compute_queen_scores(&input);
        assert_eq!(scores.lay_eggs, 0.0);
    }

    #[test]
    fn resting_decay_multiplier_is_half() {
        assert_eq!(resting_decay_multiplier(), 0.5);
    }

    // ── Phase 5: hunger signal tests ──

    #[test]
    fn hunger_signal_scales_with_hunger() {
        let fed = queen_hunger_signal(0.0, 1.0);
        let starving = queen_hunger_signal(1.0, 1.0);
        assert!(starving > fed, "starving {starving} should exceed fed {fed}");
    }

    #[test]
    fn hunger_signal_has_baseline_when_fed() {
        let signal = queen_hunger_signal(0.0, 1.0);
        assert!(signal > 0.1, "fed queen should still emit baseline signal, got {signal}");
        assert!(signal < 0.25, "fed queen signal should be low, got {signal}");
    }

    #[test]
    fn hunger_signal_reaches_max_when_starving() {
        let signal = queen_hunger_signal(1.0, 1.0);
        assert!((signal - 1.0).abs() < 0.01, "starving queen should emit max signal, got {signal}");
    }

    #[test]
    fn hunger_signal_respects_max_strength() {
        let signal = queen_hunger_signal(1.0, 0.5);
        assert!(signal <= 0.5, "signal {signal} should not exceed max_strength 0.5");
    }
}
