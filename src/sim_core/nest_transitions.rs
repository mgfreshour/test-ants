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
