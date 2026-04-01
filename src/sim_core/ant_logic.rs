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
}
