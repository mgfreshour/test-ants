use bevy::prelude::*;

use crate::components::pheromone::PheromoneType;

#[derive(Resource)]
pub struct PheromoneConfig {
    pub evaporation_rate: f32,
    pub diffusion_rate: f32,
    pub deposit_amount: f32,
    pub max_intensity: f32,
}

impl Default for PheromoneConfig {
    fn default() -> Self {
        Self {
            evaporation_rate: 0.02,
            diffusion_rate: 0.005,
            deposit_amount: 1.0,
            max_intensity: 100.0,
        }
    }
}

/// Grid storing pheromone intensities for all types.
/// Each cell holds [home, food, alarm, trail] as f32 values.
#[derive(Resource)]
pub struct PheromoneGrid {
    pub width: usize,
    pub height: usize,
    pub cell_size: f32,
    cells: Vec<[f32; PheromoneType::COUNT]>,
}

impl PheromoneGrid {
    pub fn new(world_width: f32, world_height: f32, cell_size: f32) -> Self {
        let width = (world_width / cell_size).ceil() as usize;
        let height = (world_height / cell_size).ceil() as usize;
        Self {
            width,
            height,
            cell_size,
            cells: vec![[0.0; PheromoneType::COUNT]; width * height],
        }
    }

    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn world_to_grid(&self, pos: bevy::math::Vec2) -> Option<(usize, usize)> {
        let gx = (pos.x / self.cell_size).floor() as i32;
        let gy = (pos.y / self.cell_size).floor() as i32;
        if gx >= 0 && gy >= 0 && (gx as usize) < self.width && (gy as usize) < self.height {
            Some((gx as usize, gy as usize))
        } else {
            None
        }
    }

    pub fn get(&self, x: usize, y: usize, ptype: PheromoneType) -> f32 {
        if x < self.width && y < self.height {
            self.cells[self.index(x, y)][ptype.index()]
        } else {
            0.0
        }
    }

    pub fn get_all(&self, x: usize, y: usize) -> [f32; PheromoneType::COUNT] {
        if x < self.width && y < self.height {
            self.cells[self.index(x, y)]
        } else {
            [0.0; PheromoneType::COUNT]
        }
    }

    pub fn deposit(&mut self, x: usize, y: usize, ptype: PheromoneType, amount: f32, max: f32) {
        if x < self.width && y < self.height {
            let idx = self.index(x, y);
            self.cells[idx][ptype.index()] = (self.cells[idx][ptype.index()] + amount).min(max);
        }
    }

    pub fn evaporate(&mut self, rate: f32) {
        let decay = 1.0 - rate;
        for cell in &mut self.cells {
            for val in cell.iter_mut() {
                *val *= decay;
                if *val < 0.001 {
                    *val = 0.0;
                }
            }
        }
    }

    pub fn diffuse(&mut self, rate: f32, max: f32) {
        let len = self.cells.len();
        let mut deltas = vec![[0.0f32; PheromoneType::COUNT]; len];

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.index(x, y);
                let cell = self.cells[idx];

                let mut neighbor_count = 0u32;
                let mut neighbor_sum = [0.0f32; PheromoneType::COUNT];

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
                        {
                            let nidx = self.index(nx as usize, ny as usize);
                            for i in 0..PheromoneType::COUNT {
                                neighbor_sum[i] += self.cells[nidx][i];
                            }
                            neighbor_count += 1;
                        }
                    }
                }

                if neighbor_count > 0 {
                    for i in 0..PheromoneType::COUNT {
                        let avg = neighbor_sum[i] / neighbor_count as f32;
                        deltas[idx][i] = rate * (avg - cell[i]);
                    }
                }
            }
        }

        for (idx, delta) in deltas.iter().enumerate() {
            for i in 0..PheromoneType::COUNT {
                self.cells[idx][i] = (self.cells[idx][i] + delta[i]).clamp(0.0, max);
            }
        }
    }

    /// Sample the 8 neighbors and return weighted direction toward highest concentration
    pub fn sense_gradient(&self, x: usize, y: usize, ptype: PheromoneType) -> bevy::math::Vec2 {
        let mut gradient = bevy::math::Vec2::ZERO;

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
                {
                    let intensity = self.get(nx as usize, ny as usize, ptype);
                    gradient += bevy::math::Vec2::new(dx as f32, dy as f32) * intensity;
                }
            }
        }

        gradient
    }

    /// Total pheromone at a cell across all types (for overlay intensity)
    pub fn total_intensity(&self, x: usize, y: usize) -> f32 {
        if x < self.width && y < self.height {
            let idx = self.index(x, y);
            self.cells[idx].iter().sum()
        } else {
            0.0
        }
    }
}
