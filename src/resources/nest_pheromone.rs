use bevy::prelude::*;

use crate::components::nest::{CellType, ChamberKind};
use crate::resources::nest::{NestGrid, NEST_HEIGHT, NEST_WIDTH};

/// Number of chamber identity label types.
pub const CHAMBER_LABEL_COUNT: usize = 5;

/// Indices into the chamber_labels array.
pub const LABEL_BROOD: usize = 0;
pub const LABEL_FOOD_STORAGE: usize = 1;
pub const LABEL_QUEEN: usize = 2;
pub const LABEL_MIDDEN: usize = 3;
pub const LABEL_ENTRANCE: usize = 4;

/// Per-cell pheromone data for the underground nest.
#[derive(Clone, Default)]
pub struct NestCellPheromones {
    /// Chemical road-signs identifying chamber function.
    /// [Brood, FoodStorage, Queen, Midden, Entrance]
    pub chamber_labels: [f32; CHAMBER_LABEL_COUNT],
    /// Queen pheromone signal — diffuses from queen through tunnels.
    pub queen_signal: f32,
    /// Construction pheromone — deposited at dig faces, attracts diggers.
    pub construction: f32,
    /// Brood need signal — emitted by unfed larvae.
    pub brood_need: f32,
}

/// Configuration for nest pheromone dynamics.
#[derive(Resource)]
pub struct NestPheromoneConfig {
    /// How fast chamber labels decay when unrefreshed (very slow).
    pub label_decay_rate: f32,
    /// Amount of label refreshed per ant per tick.
    pub label_refresh_amount: f32,
    /// Queen signal strength at queen's position.
    pub queen_signal_strength: f32,
    /// Queen signal decay per tick.
    pub queen_signal_decay: f32,
    /// Queen signal diffusion rate to passable neighbors.
    pub queen_signal_diffuse: f32,
    /// Construction pheromone decay rate per tick.
    pub construction_decay_rate: f32,
    /// Brood need signal decay rate per tick.
    pub brood_need_decay: f32,
    /// Brood need emission strength per unfed larva per tick.
    pub brood_need_emission: f32,
}

impl Default for NestPheromoneConfig {
    fn default() -> Self {
        Self {
            label_decay_rate: 0.0005,
            label_refresh_amount: 0.05,
            queen_signal_strength: 1.0,
            queen_signal_decay: 0.02,
            queen_signal_diffuse: 0.15,
            construction_decay_rate: 0.01,
            brood_need_decay: 0.005,
            brood_need_emission: 0.1,
        }
    }
}

/// Grid storing nest pheromone data. Sized to match NestGrid.
#[derive(Resource)]
pub struct NestPheromoneGrid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<NestCellPheromones>,
}

impl Default for NestPheromoneGrid {
    fn default() -> Self {
        Self {
            width: NEST_WIDTH,
            height: NEST_HEIGHT,
            cells: vec![NestCellPheromones::default(); NEST_WIDTH * NEST_HEIGHT],
        }
    }
}

impl NestPheromoneGrid {
    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn get(&self, x: usize, y: usize) -> &NestCellPheromones {
        if x < self.width && y < self.height {
            &self.cells[self.index(x, y)]
        } else {
            // Return a static default for out-of-bounds
            static DEFAULT: NestCellPheromones = NestCellPheromones {
                chamber_labels: [0.0; CHAMBER_LABEL_COUNT],
                queen_signal: 0.0,
                construction: 0.0,
                brood_need: 0.0,
            };
            &DEFAULT
        }
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut NestCellPheromones> {
        if x < self.width && y < self.height {
            let idx = self.index(x, y);
            Some(&mut self.cells[idx])
        } else {
            None
        }
    }

    /// Seed initial chamber labels from the NestGrid layout.
    pub fn seed_from_grid(&mut self, grid: &NestGrid) {
        for y in 0..grid.height {
            for x in 0..grid.width {
                let cell = grid.get(x, y);
                if let CellType::Chamber(kind) = cell {
                    let label_idx = chamber_kind_to_label(kind);
                    if let Some(phero) = self.get_mut(x, y) {
                        phero.chamber_labels[label_idx] = 1.0;
                    }
                }
                // Mark entrance tunnel top cells
                if cell == CellType::Tunnel && y <= 1 {
                    if let Some(phero) = self.get_mut(x, y) {
                        phero.chamber_labels[LABEL_ENTRANCE] = 1.0;
                    }
                }
            }
        }
    }

    /// Decay all pheromone layers.
    pub fn decay(&mut self, config: &NestPheromoneConfig) {
        for cell in &mut self.cells {
            // Chamber labels decay very slowly
            for label in &mut cell.chamber_labels {
                *label *= 1.0 - config.label_decay_rate;
                if *label < 0.001 {
                    *label = 0.0;
                }
            }
            // Queen signal decays
            cell.queen_signal *= 1.0 - config.queen_signal_decay;
            if cell.queen_signal < 0.001 {
                cell.queen_signal = 0.0;
            }
            // Construction pheromone decays fast
            cell.construction *= 1.0 - config.construction_decay_rate;
            if cell.construction < 0.001 {
                cell.construction = 0.0;
            }
            // Brood need decays
            cell.brood_need *= 1.0 - config.brood_need_decay;
            if cell.brood_need < 0.001 {
                cell.brood_need = 0.0;
            }
        }
    }

    /// Diffuse queen signal through passable cells only.
    pub fn diffuse_queen_signal(&mut self, grid: &NestGrid, rate: f32) {
        let len = self.cells.len();
        let mut deltas = vec![0.0f32; len];

        for y in 0..self.height {
            for x in 0..self.width {
                if !grid.get(x, y).is_passable() {
                    continue;
                }
                let idx = self.index(x, y);
                let val = self.cells[idx].queen_signal;

                let mut neighbor_count = 0u32;
                let mut neighbor_sum = 0.0f32;

                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0
                            && ny >= 0
                            && (nx as usize) < self.width
                            && (ny as usize) < self.height
                            && grid.get(nx as usize, ny as usize).is_passable()
                        {
                            let nidx = self.index(nx as usize, ny as usize);
                            neighbor_sum += self.cells[nidx].queen_signal;
                            neighbor_count += 1;
                        }
                    }
                }

                if neighbor_count > 0 {
                    let avg = neighbor_sum / neighbor_count as f32;
                    deltas[idx] = rate * (avg - val);
                }
            }
        }

        for (idx, &delta) in deltas.iter().enumerate() {
            self.cells[idx].queen_signal = (self.cells[idx].queen_signal + delta).max(0.0).min(1.0);
        }
    }

    /// Get the strongest chamber label direction from a position (for navigation).
    pub fn sense_chamber_label(
        &self,
        x: usize,
        y: usize,
        label_idx: usize,
        radius: i32,
    ) -> Option<(usize, usize)> {
        let mut best_pos = None;
        let mut best_val = 0.0f32;

        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0
                    && ny >= 0
                    && (nx as usize) < self.width
                    && (ny as usize) < self.height
                {
                    let cell = self.get(nx as usize, ny as usize);
                    if cell.chamber_labels[label_idx] > best_val {
                        best_val = cell.chamber_labels[label_idx];
                        best_pos = Some((nx as usize, ny as usize));
                    }
                }
            }
        }

        best_pos
    }
}

/// Map ChamberKind to label index.
pub fn chamber_kind_to_label(kind: ChamberKind) -> usize {
    match kind {
        ChamberKind::Brood => LABEL_BROOD,
        ChamberKind::FoodStorage => LABEL_FOOD_STORAGE,
        ChamberKind::Queen => LABEL_QUEEN,
        ChamberKind::Midden => LABEL_MIDDEN,
    }
}
