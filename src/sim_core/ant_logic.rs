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
        assert_eq!(h, 0.2);
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
}
