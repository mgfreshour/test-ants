use bevy::prelude::*;
use std::collections::HashMap;

const CELL_SIZE: f32 = 32.0;

#[derive(Resource, Default)]
pub struct SpatialGrid {
    cells: HashMap<(i32, i32), Vec<Entity>>,
}

impl SpatialGrid {
    pub fn clear(&mut self) {
        self.cells.clear();
    }

    pub fn insert(&mut self, entity: Entity, position: Vec2) {
        let key = Self::cell_key(position);
        self.cells.entry(key).or_default().push(entity);
    }

    pub fn query_radius(&self, position: Vec2, radius: f32) -> Vec<Entity> {
        let mut results = Vec::new();
        let cells_to_check = (radius / CELL_SIZE).ceil() as i32 + 1;
        let center = Self::cell_key(position);

        for dx in -cells_to_check..=cells_to_check {
            for dy in -cells_to_check..=cells_to_check {
                let key = (center.0 + dx, center.1 + dy);
                if let Some(entities) = self.cells.get(&key) {
                    results.extend(entities);
                }
            }
        }
        results
    }

    fn cell_key(position: Vec2) -> (i32, i32) {
        (
            (position.x / CELL_SIZE).floor() as i32,
            (position.y / CELL_SIZE).floor() as i32,
        )
    }
}
