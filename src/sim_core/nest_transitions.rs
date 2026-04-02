use crate::components::nest::{AttendStep, DigStep, FeedStep, HaulStep, MoveBroodStep, NestTask};

pub fn at_destination(path_done: bool, has_no_path: bool) -> bool {
    path_done || has_no_path
}

pub fn next_feed_step_on_arrival(step: FeedStep, path_exists: bool) -> Option<FeedStep> {
    match (step, path_exists) {
        (FeedStep::GoToStorage, true) => Some(FeedStep::PickUpFood),
        (FeedStep::GoToBrood, true) => Some(FeedStep::FindLarva),
        _ => None,
    }
}

pub fn next_move_brood_step_on_arrival(step: MoveBroodStep, path_exists: bool) -> Option<MoveBroodStep> {
    match (step, path_exists) {
        (MoveBroodStep::GoToQueen, true) => Some(MoveBroodStep::PickUpBrood),
        (MoveBroodStep::GoToBrood, true) => Some(MoveBroodStep::PlaceBrood),
        _ => None,
    }
}

pub fn next_haul_step_on_arrival(step: HaulStep, path_exists: bool) -> Option<HaulStep> {
    match (step, path_exists) {
        (HaulStep::GoToEntrance, true) => Some(HaulStep::PickUpFood),
        (HaulStep::GoToStorage, true) => Some(HaulStep::DropFood),
        _ => None,
    }
}

pub fn next_attend_step_on_arrival(step: AttendStep, path_exists: bool) -> Option<AttendStep> {
    match (step, path_exists) {
        (AttendStep::GoToStorage, true) => Some(AttendStep::PickUpFood),
        (AttendStep::GoToQueen, true) => Some(AttendStep::FeedQueen),
        _ => None,
    }
}

pub fn next_dig_step_on_arrival(step: DigStep, path_exists: bool) -> Option<DigStep> {
    match (step, path_exists) {
        (DigStep::GoToFace, true) => Some(DigStep::Excavate),
        _ => None,
    }
}

/// Compute effective construction pheromone decay rate scaled by humidity.
///
/// - humidity 0.0 → 2x base decay (dry, pheromone fades fast, ants spread out → larger chambers)
/// - humidity 0.5 → 1x base decay (neutral)
/// - humidity 1.0 → 0.2x base decay (humid, pheromone persists, ants cluster → smaller chambers)
pub fn humidity_scaled_decay(base_decay: f32, humidity: f32) -> f32 {
    let scale = 1.0 + (0.5 - humidity) * 2.0; // [0.0, 2.0]
    (base_decay * scale).clamp(0.0, 1.0)
}

/// Compute construction pheromone deposit rate scaled by humidity.
///
/// Humid conditions boost deposits (ants cluster more tightly).
pub fn humidity_scaled_deposit(base_deposit: f32, humidity: f32) -> f32 {
    let boost = 0.5 + humidity; // [0.5, 1.5]
    base_deposit * boost
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_step_progresses_on_completed_path() {
        assert_eq!(
            next_feed_step_on_arrival(FeedStep::GoToStorage, true),
            Some(FeedStep::PickUpFood)
        );
        assert_eq!(
            next_feed_step_on_arrival(FeedStep::GoToBrood, true),
            Some(FeedStep::FindLarva)
        );
        assert_eq!(next_feed_step_on_arrival(FeedStep::GoToStorage, false), None);
    }

    #[test]
    fn move_brood_step_progresses_on_completed_path() {
        assert_eq!(
            next_move_brood_step_on_arrival(MoveBroodStep::GoToQueen, true),
            Some(MoveBroodStep::PickUpBrood)
        );
        assert_eq!(
            next_move_brood_step_on_arrival(MoveBroodStep::GoToBrood, true),
            Some(MoveBroodStep::PlaceBrood)
        );
    }

    #[test]
    fn haul_step_progresses_on_completed_path() {
        assert_eq!(
            next_haul_step_on_arrival(HaulStep::GoToEntrance, true),
            Some(HaulStep::PickUpFood)
        );
        assert_eq!(
            next_haul_step_on_arrival(HaulStep::GoToStorage, true),
            Some(HaulStep::DropFood)
        );
    }

    #[test]
    fn attend_step_progresses_on_completed_path() {
        assert_eq!(
            next_attend_step_on_arrival(AttendStep::GoToStorage, true),
            Some(AttendStep::PickUpFood)
        );
        assert_eq!(
            next_attend_step_on_arrival(AttendStep::GoToQueen, true),
            Some(AttendStep::FeedQueen)
        );
    }

    #[test]
    fn dig_step_progresses_on_completed_path() {
        assert_eq!(
            next_dig_step_on_arrival(DigStep::GoToFace, true),
            Some(DigStep::Excavate)
        );
        assert_eq!(next_dig_step_on_arrival(DigStep::Excavate, true), None);
    }

    // ── NestTask::is_carrying tests ────────────────────────────────

    #[test]
    fn feed_larva_carrying_after_pickup() {
        let task = NestTask::FeedLarva { step: FeedStep::GoToStorage, target_larva: None };
        assert!(!task.is_carrying());
        let task = NestTask::FeedLarva { step: FeedStep::PickUpFood, target_larva: None };
        assert!(!task.is_carrying());
        let task = NestTask::FeedLarva { step: FeedStep::GoToBrood, target_larva: None };
        assert!(task.is_carrying());
        let task = NestTask::FeedLarva { step: FeedStep::DeliverFood, target_larva: None };
        assert!(task.is_carrying());
    }

    #[test]
    fn haul_food_carrying_after_pickup() {
        assert!(!NestTask::HaulFood { step: HaulStep::GoToEntrance }.is_carrying());
        assert!(!NestTask::HaulFood { step: HaulStep::PickUpFood }.is_carrying());
        assert!(NestTask::HaulFood { step: HaulStep::GoToStorage }.is_carrying());
        assert!(NestTask::HaulFood { step: HaulStep::DropFood }.is_carrying());
    }

    #[test]
    fn dig_carrying_soil_to_midden() {
        assert!(!NestTask::Dig { step: DigStep::GoToFace, target_cell: None, dig_timer: 0.0 }.is_carrying());
        assert!(!NestTask::Dig { step: DigStep::Excavate, target_cell: None, dig_timer: 0.0 }.is_carrying());
        assert!(NestTask::Dig { step: DigStep::GoToMidden, target_cell: None, dig_timer: 0.0 }.is_carrying());
        assert!(NestTask::Dig { step: DigStep::DropSoil, target_cell: None, dig_timer: 0.0 }.is_carrying());
    }

    #[test]
    fn idle_never_carrying() {
        assert!(!NestTask::Idle { timer: 0.0 }.is_carrying());
    }

    // ── Humidity scaling tests ─────────────────────────────────────

    #[test]
    fn humidity_neutral_returns_base_decay() {
        let result = humidity_scaled_decay(0.01, 0.5);
        assert!((result - 0.01).abs() < 1e-6);
    }

    #[test]
    fn humidity_dry_doubles_decay() {
        let result = humidity_scaled_decay(0.01, 0.0);
        assert!((result - 0.02).abs() < 1e-6);
    }

    #[test]
    fn humidity_humid_near_zero_decay() {
        let result = humidity_scaled_decay(0.01, 1.0);
        // scale = 1.0 + (0.5 - 1.0) * 2.0 = 0.0 → effective = 0.0
        assert!((result - 0.0).abs() < 1e-6);
    }

    #[test]
    fn humidity_deposit_neutral() {
        let result = humidity_scaled_deposit(0.15, 0.5);
        assert!((result - 0.15).abs() < 1e-6);
    }

    #[test]
    fn humidity_deposit_humid_boosts() {
        let result = humidity_scaled_deposit(0.15, 1.0);
        // boost = 0.5 + 1.0 = 1.5 → 0.15 * 1.5 = 0.225
        assert!((result - 0.225).abs() < 1e-6);
    }

    #[test]
    fn humidity_deposit_dry_reduces() {
        let result = humidity_scaled_deposit(0.15, 0.0);
        // boost = 0.5 + 0.0 = 0.5 → 0.15 * 0.5 = 0.075
        assert!((result - 0.075).abs() < 1e-6);
    }
}
