use bevy::prelude::*;

use crate::components::pheromone::PheromoneType;

#[derive(Resource)]
pub struct PheromoneConfig {
    /// Per-type evaporation rates, indexed by PheromoneType::index()
    pub evaporation_rates: [f32; PheromoneType::COUNT],
    pub diffusion_rate: f32,
    /// Per-type base deposit amounts, indexed by PheromoneType::index()
    pub deposit_amounts: [f32; PheromoneType::COUNT],
    pub max_intensity: f32,
}

impl PheromoneConfig {
    pub fn evaporation_rate(&self, ptype: PheromoneType) -> f32 {
        self.evaporation_rates[ptype.index()]
    }

    pub fn deposit_amount(&self, ptype: PheromoneType) -> f32 {
        self.deposit_amounts[ptype.index()]
    }
}

impl Default for PheromoneConfig {
    fn default() -> Self {
        Self {
            //                      Home     Food     Alarm    Trail
            evaporation_rates: [0.0005,  0.0002,  0.0005,  0.0005],
            diffusion_rate: 0.005,
            deposit_amounts:   [3.0,     8.0,     2.5,     2.5],
            max_intensity: 200.0,
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

    pub fn evaporate(&mut self, rates: &[f32; PheromoneType::COUNT]) {
        let decays: [f32; PheromoneType::COUNT] = std::array::from_fn(|i| 1.0 - rates[i]);
        for cell in &mut self.cells {
            for (i, val) in cell.iter_mut().enumerate() {
                *val *= decays[i];
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

    /// Sample cells within `radius` and return weighted direction toward highest concentration.
    /// `forward` is the ant's current heading — only cells within a forward cone are
    /// considered, preventing ants from sensing their own trail behind them.
    pub fn sense_gradient(
        &self,
        x: usize,
        y: usize,
        ptype: PheromoneType,
        forward: bevy::math::Vec2,
        radius: i32,
    ) -> bevy::math::Vec2 {
        let mut gradient = bevy::math::Vec2::ZERO;
        let fwd = if forward.length_squared() > 0.001 {
            forward.normalize()
        } else {
            bevy::math::Vec2::X
        };

        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let cell_dir = bevy::math::Vec2::new(dx as f32, dy as f32).normalize();
                let dot = fwd.dot(cell_dir);

                if dot < -0.25 {
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
                    if intensity < 0.001 {
                        continue;
                    }
                    let cone_weight = (dot + 0.25) / 1.25;
                    let dist = ((dx * dx + dy * dy) as f32).sqrt();
                    let dist_weight = 1.0 / (1.0 + dist);
                    gradient += cell_dir * intensity * cone_weight * dist_weight;
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
