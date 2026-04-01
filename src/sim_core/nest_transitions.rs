use crate::components::nest::{AttendStep, DigStep, FeedStep, HaulStep, MoveBroodStep};

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
}
