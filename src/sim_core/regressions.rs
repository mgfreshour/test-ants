use std::collections::HashMap;

pub fn should_reset_orphaned_returner(is_returning: bool, has_carried_item: bool) -> bool {
    is_returning && !has_carried_item
}

pub fn should_enter_nest(
    current_underground: usize,
    desired_underground: usize,
    is_foraging: bool,
    random_roll: f32,
    enter_chance: f32,
) -> bool {
    current_underground < desired_underground && is_foraging && random_roll <= enter_chance
}

pub fn select_available_dig_faces(
    dig_faces: &[(usize, usize)],
    dig_target_counts: &HashMap<(usize, usize), usize>,
    max_targeters_per_face: usize,
) -> Vec<(usize, usize)> {
    let available: Vec<(usize, usize)> = dig_faces
        .iter()
        .copied()
        .filter(|face| dig_target_counts.get(face).copied().unwrap_or(0) < max_targeters_per_face)
        .collect();

    if available.is_empty() {
        dig_faces.to_vec()
    } else {
        available
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn orphaned_returners_are_reset() {
        assert!(should_reset_orphaned_returner(true, false));
        assert!(!should_reset_orphaned_returner(true, true));
        assert!(!should_reset_orphaned_returner(false, false));
    }

    #[test]
    fn portal_entry_respects_capacity_state_and_probability() {
        assert!(should_enter_nest(2, 5, true, 0.01, 0.02));
        assert!(!should_enter_nest(5, 5, true, 0.01, 0.02));
        assert!(!should_enter_nest(2, 5, false, 0.01, 0.02));
        assert!(!should_enter_nest(2, 5, true, 0.5, 0.02));
    }

    #[test]
    fn dig_face_filter_falls_back_when_all_faces_are_busy() {
        let faces = vec![(1, 1), (2, 2)];
        let mut counts = HashMap::new();
        counts.insert((1, 1), 5);
        counts.insert((2, 2), 7);

        let selected = select_available_dig_faces(&faces, &counts, 5);
        assert_eq!(selected, faces);
    }
}
