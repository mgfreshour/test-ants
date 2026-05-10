use bevy::math::Vec2;
use crate::resources::surface_grid::SurfaceGrid;

pub const WALL_AVOID_WEIGHT: f32 = 1.5;
pub const WHISKER_LENGTH: f32 = 32.0;
pub const WHISKER_SPREAD: f32 = 0.4;

pub fn compute_wall_avoidance(
    pos: Vec2,
    heading: Vec2,
    grid: &SurfaceGrid,
    whisker_length: f32,
    whisker_spread: f32,
) -> Vec2 {
    if heading.length_squared() < 0.001 {
        return Vec2::ZERO;
    }

    let angle = heading.y.atan2(heading.x);
    let left_angle = angle + whisker_spread;
    let right_angle = angle - whisker_spread;

    let center_dir = heading.normalize();
    let left_dir = Vec2::new(left_angle.cos(), left_angle.sin());
    let right_dir = Vec2::new(right_angle.cos(), right_angle.sin());

    let center_hit = cast_whisker(pos, center_dir, whisker_length, grid);
    let left_hit = cast_whisker(pos, left_dir, whisker_length, grid);
    let right_hit = cast_whisker(pos, right_dir, whisker_length, grid);

    match (center_hit, left_hit, right_hit) {
        (None, None, None) => Vec2::ZERO,

        (Some(cd), None, None) => {
            let perp = Vec2::new(-center_dir.y, center_dir.x);
            perp * proximity_strength(cd, whisker_length)
        }

        (_, None, Some(rd)) => {
            left_dir * proximity_strength(rd, whisker_length)
        }

        (_, Some(ld), None) => {
            right_dir * proximity_strength(ld, whisker_length)
        }

        (_, Some(ld), Some(rd)) => {
            if ld < rd {
                let perp = Vec2::new(center_dir.y, -center_dir.x);
                perp * proximity_strength(ld.min(rd), whisker_length)
            } else {
                let perp = Vec2::new(-center_dir.y, center_dir.x);
                perp * proximity_strength(ld.min(rd), whisker_length)
            }
        }
    }
}

fn proximity_strength(hit_dist: f32, max_dist: f32) -> f32 {
    (1.0 - hit_dist / max_dist).max(0.0)
}

fn cast_whisker(
    origin: Vec2,
    direction: Vec2,
    length: f32,
    grid: &SurfaceGrid,
) -> Option<f32> {
    let step = grid.cell_size * 0.5;
    let steps = (length / step).ceil() as usize;

    for i in 1..=steps {
        let dist = step * i as f32;
        let sample = origin + direction * dist;
        if grid.is_blocked_world(sample) {
            return Some(dist);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_with_wall() -> SurfaceGrid {
        let mut grid = SurfaceGrid::new(10, 10, 16.0);
        // Wall at column 5
        for y in 0..10 {
            grid.set(5, y, crate::resources::surface_grid::SurfaceCell::Blocked);
        }
        grid
    }

    #[test]
    fn no_avoidance_in_open_space() {
        let grid = SurfaceGrid::new(10, 10, 16.0);
        let force = compute_wall_avoidance(
            Vec2::new(80.0, 80.0),
            Vec2::new(1.0, 0.0),
            &grid, 32.0, 0.4,
        );
        assert_eq!(force, Vec2::ZERO);
    }

    #[test]
    fn avoidance_when_heading_toward_wall() {
        let grid = grid_with_wall();
        // Ant at x=64 (cell 4), heading right toward wall at cell 5 (x=80)
        let force = compute_wall_avoidance(
            Vec2::new(64.0, 80.0),
            Vec2::new(1.0, 0.0),
            &grid, 32.0, 0.4,
        );
        assert!(force.length() > 0.0, "Should produce avoidance force");
        // Force should have a y-component (steering away from wall)
        assert!(force.y.abs() > 0.01, "Force should steer laterally");
    }

    #[test]
    fn no_avoidance_heading_away_from_wall() {
        // Larger grid so whiskers don't hit out-of-bounds edges
        let mut grid = SurfaceGrid::new(20, 20, 16.0);
        for y in 0..20 {
            grid.set(15, y, crate::resources::surface_grid::SurfaceCell::Blocked);
        }
        // Ant at cell 5, heading left — wall at cell 15 is far behind
        let force = compute_wall_avoidance(
            Vec2::new(80.0, 160.0),
            Vec2::new(-1.0, 0.0),
            &grid, 32.0, 0.4,
        );
        assert!(force.length() < 0.01, "Should produce negligible force: {force:?}");
    }

    #[test]
    fn stronger_force_when_closer() {
        let grid = grid_with_wall();
        let far = compute_wall_avoidance(
            Vec2::new(56.0, 80.0),
            Vec2::new(1.0, 0.0),
            &grid, 32.0, 0.4,
        );
        let close = compute_wall_avoidance(
            Vec2::new(72.0, 80.0),
            Vec2::new(1.0, 0.0),
            &grid, 32.0, 0.4,
        );
        assert!(close.length() > far.length(), "Closer ant should get stronger force");
    }

    #[test]
    fn zero_heading_produces_no_force() {
        let grid = grid_with_wall();
        let force = compute_wall_avoidance(
            Vec2::new(72.0, 80.0),
            Vec2::ZERO,
            &grid, 32.0, 0.4,
        );
        assert_eq!(force, Vec2::ZERO);
    }
}
