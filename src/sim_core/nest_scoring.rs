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
        let hunger_urgency = 0.3 + input.queen_hunger * 0.7;
        let crowding = 1.0 / (1.0 + input.current_queen_attendants as f32 * 1.5);
        0.8 * nursing_affinity * hunger_urgency * (0.3 + input.queen_signal * 0.7) * crowding
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
}
