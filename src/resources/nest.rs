use bevy::prelude::*;

use crate::components::nest::{CellType, ChamberKind};

pub const NEST_WIDTH: usize = 60;
pub const NEST_HEIGHT: usize = 40;
pub const NEST_CELL_SIZE: f32 = 16.0;

#[derive(Component)]
pub struct NestGrid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<CellType>>,
}

impl Default for NestGrid {
    fn default() -> Self {
        let mut cells = vec![vec![CellType::Soil; NEST_WIDTH]; NEST_HEIGHT];

        // Surface layer (top 2 rows = soft soil)
        for x in 0..NEST_WIDTH {
            cells[0][x] = CellType::SoftSoil;
            cells[1][x] = CellType::SoftSoil;
        }

        // Rock layer at bottom
        for x in 0..NEST_WIDTH {
            cells[NEST_HEIGHT - 1][x] = CellType::Rock;
            cells[NEST_HEIGHT - 2][x] = CellType::Rock;
        }

        // Entrance tunnel from surface (column 30, rows 0-6)
        let cx = NEST_WIDTH / 2;
        for y in 0..7 {
            cells[y][cx] = CellType::Tunnel;
        }

        // Food storage chamber (near surface, left of tunnel)
        for y in 5..8 {
            for x in (cx - 3)..(cx +1) {
                cells[y][x] = CellType::Chamber(ChamberKind::FoodStorage);
            }
        }

        // Brood chamber (mid depth, right of tunnel)
        for y in 8..12 {
            for x in (cx + 2)..(cx + 7) {
                cells[y][x] = CellType::Chamber(ChamberKind::Brood);
            }
        }

        // Connecting tunnel to brood
        for y in 5..9 {
            cells[y][cx] = CellType::Tunnel;
            cells[y][cx + 1] = CellType::Tunnel;
        }

        // Queen chamber (deep)
        for y in 15..18 {
            for x in (cx - 2)..(cx + 3) {
                cells[y][x] = CellType::Chamber(ChamberKind::Queen);
            }
        }

        // Tunnel from brood to queen
        for y in 9..16 {
            cells[y][cx] = CellType::Tunnel;
        }

        // Midden (waste, far side)
        for y in 20..23 {
            for x in (cx + 6)..(cx + 10) {
                cells[y][x] = CellType::Chamber(ChamberKind::Midden);
            }
        }
        // Tunnel to midden
        for y in 17..21 {
            cells[y][cx + 2] = CellType::Tunnel;
        }
        cells[20][cx + 3] = CellType::Tunnel;
        cells[20][cx + 4] = CellType::Tunnel;
        cells[20][cx + 5] = CellType::Tunnel;

        Self {
            width: NEST_WIDTH,
            height: NEST_HEIGHT,
            cells,
        }
    }
}

impl NestGrid {
    pub fn get(&self, x: usize, y: usize) -> CellType {
        if y < self.height && x < self.width {
            self.cells[y][x]
        } else {
            CellType::Rock
        }
    }

    /// Mutate a cell (e.g., excavate soil -> tunnel). Returns true if changed.
    pub fn set(&mut self, x: usize, y: usize, cell_type: CellType) -> bool {
        if y < self.height && x < self.width {
            self.cells[y][x] = cell_type;
            true
        } else {
            false
        }
    }

    /// Find diggable cells adjacent to passable cells (dig faces).
    pub fn find_dig_faces(&self) -> Vec<(usize, usize)> {
        let mut faces = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if !self.get(x, y).is_diggable() {
                    continue;
                }
                let has_passable_neighbor = [(-1i32, 0), (1, 0), (0, -1i32), (0, 1)]
                    .iter()
                    .any(|&(dx, dy)| {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        nx >= 0
                            && ny >= 0
                            && (nx as usize) < self.width
                            && (ny as usize) < self.height
                            && self.get(nx as usize, ny as usize).is_passable()
                    });
                if has_passable_neighbor {
                    faces.push((x, y));
                }
            }
        }
        faces
    }
}

/// Player-designated dig targets. Cells in this set get a utility scoring boost.
#[derive(Component, Default)]
pub struct PlayerDigZones {
    pub cells: std::collections::HashSet<(usize, usize)>,
}
