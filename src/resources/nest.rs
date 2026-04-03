use bevy::prelude::*;

use crate::components::nest::{CellType, ChamberKind, intgrid_to_celltype};

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
    /// Build a NestGrid from LDtk IntGrid tile data.
    /// Takes (GridCoords.x, GridCoords.y, IntGrid value) tuples.
    /// GridCoords uses y=0 at bottom (Bevy convention); NestGrid uses y=0 at top.
    pub fn from_intgrid(width: usize, height: usize, tiles: &[(i32, i32, i32)]) -> Self {
        let mut cells = vec![vec![CellType::Soil; width]; height];
        for &(gx, gy, value) in tiles {
            // Flip Y: GridCoords y=0 is bottom, NestGrid y=0 is top
            let nest_y = (height as i32 - 1 - gy) as usize;
            let nest_x = gx as usize;
            if nest_x < width && nest_y < height {
                cells[nest_y][nest_x] = intgrid_to_celltype(value);
            }
        }
        Self { width, height, cells }
    }

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

#[derive(Component, Default)]
pub struct TileStackRegistry {
    pub stacks: std::collections::HashMap<(usize, usize), Vec<Entity>>,
}

impl TileStackRegistry {
    pub fn push(&mut self, grid_pos: (usize, usize), entity: Entity) -> Option<u8> {
        let stack = self.stacks.entry(grid_pos).or_insert_with(Vec::new);
        if stack.len() >= 5 { return None; }
        stack.push(entity);
        Some((stack.len() - 1) as u8)
    }

    pub fn remove(&mut self, grid_pos: (usize, usize), entity: Entity) {
        if let Some(stack) = self.stacks.get_mut(&grid_pos) {
            stack.retain(|&e| e != entity);
            if stack.is_empty() { self.stacks.remove(&grid_pos); }
        }
    }

    pub fn find_available_tile(&self, grid: &NestGrid, chamber: ChamberKind) -> Option<(usize, usize)> {
        for y in 0..grid.height {
            for x in 0..grid.width {
                if grid.get(x, y) == CellType::Chamber(chamber) && self.count_at((x, y)) < 5 {
                    return Some((x, y));
                }
            }
        }
        None
    }

    fn count_at(&self, grid_pos: (usize, usize)) -> usize {
        self.stacks.get(&grid_pos).map_or(0, |s| s.len())
    }
}

/// Result of attempting to expand a chamber zone.
pub struct ZoneExpansion {
    pub x: usize,
    pub y: usize,
}

impl NestGrid {
    /// Find a tunnel cell suitable for expanding into a new chamber of the given kind.
    /// Prefers tunnels adjacent to existing chambers of the same type at similar depth.
    /// Returns the position of the tunnel to convert, if found.
    pub fn find_expansion_candidate(&self, chamber: ChamberKind) -> Option<ZoneExpansion> {
        let existing_cells: Vec<(usize, usize)> = (0..self.height)
            .flat_map(|y| (0..self.width).map(move |x| (x, y)))
            .filter(|&(x, y)| self.get(x, y) == CellType::Chamber(chamber))
            .collect();

        if existing_cells.is_empty() {
            return None;
        }

        let avg_y: usize = existing_cells.iter().map(|&(_, y)| y).sum::<usize>() / existing_cells.len();
        let depth_tolerance: i32 = 3;

        let mut candidates: Vec<((usize, usize), i32)> = Vec::new();

        for y in 0..self.height {
            let depth_diff = (y as i32 - avg_y as i32).abs();
            if depth_diff > depth_tolerance {
                continue;
            }

            for x in 0..self.width {
                if self.get(x, y) != CellType::Tunnel {
                    continue;
                }

                let adjacent_chamber_count = [(-1i32, 0), (1, 0), (0, -1i32), (0, 1)]
                    .iter()
                    .filter(|&&(dx, dy)| {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        nx >= 0
                            && ny >= 0
                            && (nx as usize) < self.width
                            && (ny as usize) < self.height
                            && self.get(nx as usize, ny as usize) == CellType::Chamber(chamber)
                    })
                    .count() as i32;

                let score = adjacent_chamber_count * 10 - depth_diff;
                candidates.push(((x, y), score));
            }
        }

        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        candidates.first().map(|&((x, y), _)| ZoneExpansion { x, y })
    }
}

pub fn stack_position_offset(index: u8) -> Vec2 {
    let offset = NEST_CELL_SIZE * 0.3;
    match index {
        0 => Vec2::new(0.0, 0.0),
        1 => Vec2::new(-offset, offset),
        2 => Vec2::new(offset, offset),
        3 => Vec2::new(-offset, -offset),
        4 => Vec2::new(offset, -offset),
        _ => Vec2::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_grid(width: usize, height: usize) -> NestGrid {
        NestGrid {
            width,
            height,
            cells: vec![vec![CellType::Soil; width]; height],
        }
    }

    #[test]
    fn find_expansion_returns_none_when_no_existing_chambers() {
        let grid = make_test_grid(10, 10);
        assert!(grid.find_expansion_candidate(ChamberKind::FoodStorage).is_none());
    }

    #[test]
    fn find_expansion_returns_none_when_no_tunnels_nearby() {
        let mut grid = make_test_grid(10, 10);
        grid.cells[5][5] = CellType::Chamber(ChamberKind::FoodStorage);
        assert!(grid.find_expansion_candidate(ChamberKind::FoodStorage).is_none());
    }

    #[test]
    fn find_expansion_prefers_adjacent_tunnel() {
        let mut grid = make_test_grid(10, 10);
        grid.cells[5][5] = CellType::Chamber(ChamberKind::FoodStorage);
        grid.cells[5][6] = CellType::Tunnel; // adjacent
        grid.cells[5][8] = CellType::Tunnel; // not adjacent

        let result = grid.find_expansion_candidate(ChamberKind::FoodStorage);
        assert!(result.is_some());
        let exp = result.unwrap();
        assert_eq!((exp.x, exp.y), (6, 5)); // should pick adjacent tunnel
    }

    #[test]
    fn find_expansion_respects_depth_tolerance() {
        let mut grid = make_test_grid(10, 15);
        grid.cells[5][5] = CellType::Chamber(ChamberKind::Brood);
        grid.cells[5][6] = CellType::Chamber(ChamberKind::Brood);
        // avg_y = 5, tolerance = 3, so y in [2, 8] allowed
        grid.cells[12][5] = CellType::Tunnel; // too far (depth 12, diff = 7)
        grid.cells[7][5] = CellType::Tunnel;  // within tolerance (depth 7, diff = 2)

        let result = grid.find_expansion_candidate(ChamberKind::Brood);
        assert!(result.is_some());
        let exp = result.unwrap();
        assert_eq!(exp.y, 7); // should pick tunnel within depth tolerance
    }

    #[test]
    fn find_expansion_scores_by_adjacency_and_depth() {
        let mut grid = make_test_grid(10, 10);
        // cells[y][x], so chambers at (3,5) and (5,5), tunnel at (4,5) between them
        grid.cells[5][3] = CellType::Chamber(ChamberKind::FoodStorage);
        grid.cells[5][5] = CellType::Chamber(ChamberKind::FoodStorage);
        grid.cells[5][4] = CellType::Tunnel; // adjacent to 2 chambers (at x=3 and x=5)
        grid.cells[5][2] = CellType::Tunnel; // adjacent to 1 chamber (at x=3)
        grid.cells[6][3] = CellType::Tunnel; // adjacent to 1 chamber but different y

        let result = grid.find_expansion_candidate(ChamberKind::FoodStorage);
        assert!(result.is_some());
        let exp = result.unwrap();
        // (4, 5) has 2 adjacent chambers, should score highest
        assert_eq!((exp.x, exp.y), (4, 5));
    }

    #[test]
    fn find_expansion_only_considers_matching_chamber_type() {
        let mut grid = make_test_grid(10, 10);
        grid.cells[5][5] = CellType::Chamber(ChamberKind::FoodStorage);
        grid.cells[5][6] = CellType::Tunnel;

        // Looking for Brood expansion should find nothing (no Brood chambers exist)
        assert!(grid.find_expansion_candidate(ChamberKind::Brood).is_none());

        // Looking for FoodStorage expansion should find the tunnel
        assert!(grid.find_expansion_candidate(ChamberKind::FoodStorage).is_some());
    }

    #[test]
    fn find_available_tile_returns_chamber_with_space() {
        let mut grid = make_test_grid(10, 10);
        grid.cells[5][5] = CellType::Chamber(ChamberKind::FoodStorage);

        let registry = TileStackRegistry::default();
        let result = registry.find_available_tile(&grid, ChamberKind::FoodStorage);
        assert_eq!(result, Some((5, 5)));
    }

    #[test]
    fn find_available_tile_returns_none_when_all_full() {
        let mut grid = make_test_grid(10, 10);
        grid.cells[5][5] = CellType::Chamber(ChamberKind::FoodStorage);

        let mut registry = TileStackRegistry::default();
        // Fill up with 5 items (the max per tile)
        for i in 0..5 {
            registry.push((5, 5), Entity::from_raw_u32(i).unwrap());
        }

        let result = registry.find_available_tile(&grid, ChamberKind::FoodStorage);
        assert!(result.is_none());
    }

    #[test]
    fn stack_registry_push_respects_limit() {
        let mut registry = TileStackRegistry::default();
        for i in 0..5 {
            assert!(registry.push((0, 0), Entity::from_raw_u32(i).unwrap()).is_some());
        }
        // 6th push should fail
        assert!(registry.push((0, 0), Entity::from_raw_u32(5).unwrap()).is_none());
    }

    #[test]
    fn stack_registry_remove_cleans_up_empty_stacks() {
        let mut registry = TileStackRegistry::default();
        let e = Entity::from_raw_u32(1).unwrap();
        registry.push((0, 0), e);
        assert!(registry.stacks.contains_key(&(0, 0)));

        registry.remove((0, 0), e);
        assert!(!registry.stacks.contains_key(&(0, 0)));
    }

    #[test]
    fn from_intgrid_basic_mapping() {
        // GridCoords y=0 is bottom, NestGrid y=0 is top.
        // 3x3 grid, height=3 so nest_y = 2 - GridCoords.y
        let tiles = vec![
            (0, 2, 1), // GridCoords(0,2) = top row → NestGrid(0,0) = Soil
            (1, 2, 5), // GridCoords(1,2) = top row → NestGrid(1,0) = Tunnel
            (2, 2, 4), // GridCoords(2,2) = top row → NestGrid(2,0) = Rock
            (0, 1, 2), // GridCoords(0,1) = mid row → NestGrid(0,1) = SoftSoil
            (1, 1, 6), // GridCoords(1,1) = mid row → NestGrid(1,1) = Chamber(Queen)
            (2, 1, 3), // GridCoords(2,1) = mid row → NestGrid(2,1) = Clay
            (0, 0, 7), // GridCoords(0,0) = bot row → NestGrid(0,2) = Chamber(Brood)
            (1, 0, 8), // GridCoords(1,0) = bot row → NestGrid(1,2) = Chamber(FoodStorage)
            (2, 0, 9), // GridCoords(2,0) = bot row → NestGrid(2,2) = Chamber(Midden)
        ];

        let grid = NestGrid::from_intgrid(3, 3, &tiles);
        assert_eq!(grid.width, 3);
        assert_eq!(grid.height, 3);

        // Top row (NestGrid y=0)
        assert_eq!(grid.get(0, 0), CellType::Soil);
        assert_eq!(grid.get(1, 0), CellType::Tunnel);
        assert_eq!(grid.get(2, 0), CellType::Rock);

        // Middle row (NestGrid y=1)
        assert_eq!(grid.get(0, 1), CellType::SoftSoil);
        assert_eq!(grid.get(1, 1), CellType::Chamber(ChamberKind::Queen));
        assert_eq!(grid.get(2, 1), CellType::Clay);

        // Bottom row (NestGrid y=2)
        assert_eq!(grid.get(0, 2), CellType::Chamber(ChamberKind::Brood));
        assert_eq!(grid.get(1, 2), CellType::Chamber(ChamberKind::FoodStorage));
        assert_eq!(grid.get(2, 2), CellType::Chamber(ChamberKind::Midden));
    }

    #[test]
    fn from_intgrid_matches_default_layout() {
        // Build a NestGrid from default, convert to intgrid tiles, rebuild, compare.
        let default_grid = NestGrid::default();
        let mut tiles = Vec::new();
        for y in 0..default_grid.height {
            for x in 0..default_grid.width {
                let cell = default_grid.get(x, y);
                let value = match cell {
                    CellType::Soil => 1,
                    CellType::SoftSoil => 2,
                    CellType::Clay => 3,
                    CellType::Rock => 4,
                    CellType::Tunnel => 5,
                    CellType::Chamber(ChamberKind::Queen) => 6,
                    CellType::Chamber(ChamberKind::Brood) => 7,
                    CellType::Chamber(ChamberKind::FoodStorage) => 8,
                    CellType::Chamber(ChamberKind::Midden) => 9,
                };
                // Convert nest y to GridCoords y (flip)
                let gc_y = (default_grid.height - 1 - y) as i32;
                tiles.push((x as i32, gc_y, value));
            }
        }

        let rebuilt = NestGrid::from_intgrid(default_grid.width, default_grid.height, &tiles);

        for y in 0..default_grid.height {
            for x in 0..default_grid.width {
                assert_eq!(
                    rebuilt.get(x, y),
                    default_grid.get(x, y),
                    "Mismatch at ({}, {}): rebuilt={:?}, default={:?}",
                    x, y, rebuilt.get(x, y), default_grid.get(x, y)
                );
            }
        }
    }
}
