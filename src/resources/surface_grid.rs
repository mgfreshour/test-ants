use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceCell {
    Open,
    Blocked,
}

#[derive(Resource)]
pub struct SurfaceGrid {
    pub width: usize,
    pub height: usize,
    pub cell_size: f32,
    pub cells: Vec<SurfaceCell>,
}

impl SurfaceGrid {
    pub fn new(width: usize, height: usize, cell_size: f32) -> Self {
        Self {
            width,
            height,
            cell_size,
            cells: vec![SurfaceCell::Open; width * height],
        }
    }

    pub fn get(&self, x: usize, y: usize) -> SurfaceCell {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x]
        } else {
            SurfaceCell::Blocked
        }
    }

    pub fn set(&mut self, x: usize, y: usize, cell: SurfaceCell) {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x] = cell;
        }
    }

    pub fn is_blocked(&self, x: usize, y: usize) -> bool {
        self.get(x, y) == SurfaceCell::Blocked
    }

    pub fn world_to_grid(&self, pos: Vec2) -> Option<(usize, usize)> {
        let gx = (pos.x / self.cell_size).floor() as i32;
        let gy = (pos.y / self.cell_size).floor() as i32;
        if gx >= 0 && gy >= 0 && (gx as usize) < self.width && (gy as usize) < self.height {
            Some((gx as usize, gy as usize))
        } else {
            None
        }
    }

    pub fn is_blocked_world(&self, pos: Vec2) -> bool {
        match self.world_to_grid(pos) {
            Some((x, y)) => self.is_blocked(x, y),
            None => true,
        }
    }

    pub fn from_intgrid(width: usize, height: usize, cell_size: f32, tiles: &[(i32, i32, i32)]) -> Self {
        let mut grid = Self::new(width, height, cell_size);
        for &(x, y, value) in tiles {
            if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                continue;
            }
            let cell = intgrid_to_surface_cell(value);
            grid.set(x as usize, y as usize, cell);
        }
        grid
    }
}

fn intgrid_to_surface_cell(value: i32) -> SurfaceCell {
    match value {
        1..=7 => SurfaceCell::Open,
        _ => SurfaceCell::Blocked,
    }
}

impl Default for SurfaceGrid {
    fn default() -> Self {
        Self::new(128, 128, 16.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_grid_is_all_open() {
        let grid = SurfaceGrid::new(4, 4, 16.0);
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(grid.get(x, y), SurfaceCell::Open);
            }
        }
    }

    #[test]
    fn out_of_bounds_is_blocked() {
        let grid = SurfaceGrid::new(4, 4, 16.0);
        assert_eq!(grid.get(10, 10), SurfaceCell::Blocked);
    }

    #[test]
    fn world_to_grid_conversion() {
        let grid = SurfaceGrid::new(128, 128, 16.0);
        assert_eq!(grid.world_to_grid(Vec2::new(0.0, 0.0)), Some((0, 0)));
        assert_eq!(grid.world_to_grid(Vec2::new(31.9, 31.9)), Some((1, 1)));
        assert_eq!(grid.world_to_grid(Vec2::new(2048.0, 0.0)), None);
        assert_eq!(grid.world_to_grid(Vec2::new(-1.0, 0.0)), None);
    }

    #[test]
    fn from_intgrid_maps_values() {
        let tiles = vec![
            (0, 0, 1), // grass_dark → Open
            (1, 0, 8), // rock → Blocked
            (2, 0, 3), // dirt → Open
        ];
        let grid = SurfaceGrid::from_intgrid(4, 4, 16.0, &tiles);
        assert_eq!(grid.get(0, 0), SurfaceCell::Open);
        assert_eq!(grid.get(1, 0), SurfaceCell::Blocked);
        assert_eq!(grid.get(2, 0), SurfaceCell::Open);
    }
}
