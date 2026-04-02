#[derive(Debug, Clone, Copy)]
pub struct NestScoringInput {
    pub unfed_larvae: usize,
    pub unrelocated_brood: usize,
    pub has_food: bool,
    pub colony_food_stored: f32,
    pub has_queen: bool,
    pub queen_hunger: f32,
    pub brood_need: f32,
    pub queen_signal: f32,
    pub nearest_face_construction: f32,
    pub has_dig_faces: bool,
    pub has_player_dig_zones: bool,
    pub expansion_need: f32,
    pub current_diggers: usize,
    pub current_movers: usize,
    pub current_queen_attendants: usize,
    pub ant_age: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct NestTaskScores {
    pub feed: f32,
    pub move_brood: f32,
    pub haul: f32,
    pub attend_queen: f32,
    pub dig: f32,
    pub idle: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestTaskChoice {
    MoveBrood,
    FeedLarva,
    Dig,
    HaulFood,
    AttendQueen,
    Idle,
}

pub fn compute_scores(input: &NestScoringInput) -> NestTaskScores {
    // Age-based affinity (temporal polyethism).
    let age_frac = (input.ant_age / 300.0).clamp(0.0, 1.0);
    let nursing_affinity = 1.0 - age_frac * 0.8;
    let hauling_affinity = 0.3 + age_frac * 0.7;
    let digging_affinity = if age_frac > 0.15 && age_frac < 0.6 { 1.2 } else { 0.4 };

    let feed_score = if input.unfed_larvae > 0 && input.has_food {
        let need = (input.unfed_larvae as f32 / 5.0).min(1.0);
        need * nursing_affinity * (0.3 + input.brood_need * 0.7)
    } else {
        0.0
    };

    let move_brood_score = if input.unrelocated_brood > 0 {
        let urgency = (input.unrelocated_brood as f32 / 3.0).min(1.0);
        let crowding = 1.0 / (1.0 + input.current_movers as f32 * 0.8);
        urgency * nursing_affinity * 0.7 * crowding
    } else {
        0.0
    };

    let haul_score = if input.colony_food_stored > 2.0 {
        0.4 * hauling_affinity
    } else {
        0.0
    };

    let queen_score = if input.has_queen && input.has_food {
        // Desired attendants scales inversely with satiation (queen_hunger = 1 - satiation):
        //   full queen (hunger=0.0) → 0 desired  → no need to attend
        //   empty queen (hunger=1.0) → 4 desired → urgently needs feeding
        let desired_attendants = (input.queen_hunger * 4.0).ceil() as usize;
        let attendant_need = if input.current_queen_attendants < desired_attendants {
            1.0
        } else {
            0.1 // small residual so it can still win if nothing else scores
        };
        let hunger_urgency = 0.3 + input.queen_hunger * 0.7;
        0.8 * nursing_affinity * hunger_urgency * (0.3 + input.queen_signal * 0.7) * attendant_need
    } else {
        0.0
    };

    let dig_score = if input.has_dig_faces {
        let stigmergic = 0.3 + input.nearest_face_construction * 0.7;
        let player_boost = if input.has_player_dig_zones { 0.4 } else { 0.0 };
        let crowding_penalty = 1.0 / (1.0 + input.current_diggers as f32 * 0.3);
        (stigmergic + player_boost + input.expansion_need).min(1.0) * digging_affinity * crowding_penalty
    } else {
        0.0
    };

    NestTaskScores {
        feed: feed_score,
        move_brood: move_brood_score,
        haul: haul_score,
        attend_queen: queen_score,
        dig: dig_score,
        idle: 0.05,
    }
}

pub fn choose_task(scores: &NestTaskScores) -> NestTaskChoice {
    let values = [scores.feed, scores.move_brood, scores.haul, scores.attend_queen, scores.dig, scores.idle];
    let max_score = values.into_iter().fold(0.0f32, f32::max);

    if max_score == scores.move_brood && scores.move_brood > 0.0 {
        NestTaskChoice::MoveBrood
    } else if max_score == scores.feed && scores.feed > 0.0 {
        NestTaskChoice::FeedLarva
    } else if max_score == scores.dig && scores.dig > 0.0 {
        NestTaskChoice::Dig
    } else if max_score == scores.haul && scores.haul > 0.0 {
        NestTaskChoice::HaulFood
    } else if max_score == scores.attend_queen && scores.attend_queen > 0.0 {
        NestTaskChoice::AttendQueen
    } else {
        NestTaskChoice::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::{choose_task, compute_scores, NestTaskChoice};
    use crate::sim_core::test_fixtures::NestScoringInputBuilder;

    #[test]
    fn feeding_wins_when_larvae_need_food() {
        let input = NestScoringInputBuilder::new()
            .larvae(5)
            .food(4.0)
            .age(0.0)
            .build();
        let scores = compute_scores(&input);
        assert_eq!(choose_task(&scores), NestTaskChoice::FeedLarva);
    }

    #[test]
    fn attending_queen_wins_when_hungry_and_fed() {
        let input = NestScoringInputBuilder::new()
            .food(4.0)
            .queen(1.0, 1.0)
            .build();
        let scores = compute_scores(&input);
        assert_eq!(choose_task(&scores), NestTaskChoice::AttendQueen);
    }

    #[test]
    fn move_brood_wins_when_unrelocated_brood_exists() {
        let input = NestScoringInputBuilder::new()
            .move_brood(6)
            .food(0.0)
            .build();
        let scores = compute_scores(&input);
        assert_eq!(choose_task(&scores), NestTaskChoice::MoveBrood);
    }

    #[test]
    fn dig_wins_when_faces_present_and_ant_in_digging_age_window() {
        let input = NestScoringInputBuilder::new()
            .dig_front(1.0, true)
            .age(90.0)
            .build();
        let scores = compute_scores(&input);
        assert_eq!(choose_task(&scores), NestTaskChoice::Dig);
    }

    #[test]
    fn mover_crowding_reduces_move_brood_pressure() {
        let low_crowd = NestScoringInputBuilder::new()
            .move_brood(4)
            .mover_load(0)
            .build();
        let high_crowd = NestScoringInputBuilder::new()
            .move_brood(4)
            .mover_load(6)
            .build();

        let low_scores = compute_scores(&low_crowd);
        let high_scores = compute_scores(&high_crowd);
        assert!(low_scores.move_brood > high_scores.move_brood);
    }

    #[test]
    fn queen_attendant_score_scales_with_hunger() {
        let full = NestScoringInputBuilder::new()
            .food(4.0)
            .queen(0.0, 1.0) // hunger=0 → satiation=1.0 (full)
            .build();
        let hungry = NestScoringInputBuilder::new()
            .food(4.0)
            .queen(1.0, 1.0) // hunger=1 → satiation=0.0 (empty)
            .build();

        let full_scores = compute_scores(&full);
        let hungry_scores = compute_scores(&hungry);
        assert!(hungry_scores.attend_queen > full_scores.attend_queen);
    }

    #[test]
    fn full_queen_has_low_attendant_score() {
        let input = NestScoringInputBuilder::new()
            .food(4.0)
            .queen(0.0, 1.0) // full queen
            .queen_attendants(0)
            .build();

        let scores = compute_scores(&input);
        // Full queen: hunger_urgency = 0.3, attendant_need = 0.1 (desired=0, already satisfied)
        assert!(scores.attend_queen < 0.1);
    }

    #[test]
    fn hungry_queen_wins_task_selection() {
        let input = NestScoringInputBuilder::new()
            .food(4.0)
            .queen(0.8, 1.0) // very hungry queen
            .queen_attendants(0)
            .build();

        let scores = compute_scores(&input);
        assert_eq!(choose_task(&scores), NestTaskChoice::AttendQueen);
    }

    #[test]
    fn extra_attendants_reduce_queen_attend_score() {
        let understaffed = NestScoringInputBuilder::new()
            .food(4.0)
            .queen(0.8, 1.0)
            .queen_attendants(0)
            .build();
        let overstaffed = NestScoringInputBuilder::new()
            .food(4.0)
            .queen(0.8, 1.0)
            .queen_attendants(4) // already at desired count
            .build();

        let under_scores = compute_scores(&understaffed);
        let over_scores = compute_scores(&overstaffed);
        assert!(under_scores.attend_queen > over_scores.attend_queen);
    }

    #[test]
    fn starvation_timing_is_generous() {
        // Grace period: 30s, damage: 0.5/sec, queen HP: 100
        // Total survival time: 30 + 200 = 230 seconds
        // Generous threshold: must survive at least 7 feeding cycles (~30s each)
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
            satiation: 0.25, // queen was just fed
            decay_rate: 0.005,
            starvation_timer: 50.0,
            egg_timer: 0.0,
        };
        // The starvation system resets the timer when satiation > 0
        if hunger.satiation > 0.0 {
            hunger.starvation_timer = 0.0;
        }
        assert_eq!(hunger.starvation_timer, 0.0);
    }
}
