pub fn clock_tick_step(elapsed: f32, tick: u64, delta_secs: f32, speed_multiplier: f32, paused: bool) -> (f32, u64) {
    let next_elapsed = elapsed + delta_secs * speed_multiplier;
    let next_tick = if paused { tick } else { tick + 1 };
    (next_elapsed, next_tick)
}

#[cfg(test)]
mod tests {
    use super::clock_tick_step;

    #[test]
    fn paused_clock_does_not_advance_tick() {
        let (elapsed, tick) = clock_tick_step(10.0, 42, 0.5, 0.0, true);
        assert_eq!(elapsed, 10.0);
        assert_eq!(tick, 42);
    }

    #[test]
    fn speed_multiplier_scales_elapsed() {
        let (elapsed, tick) = clock_tick_step(1.0, 9, 0.5, 4.0, false);
        assert_eq!(elapsed, 3.0);
        assert_eq!(tick, 10);
    }
}
