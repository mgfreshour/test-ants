use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

use bevy::prelude::*;

use crate::resources::nest::NestGrid;

/// Grid position type alias.
pub type GridPos = (usize, usize);

/// A* node for the priority queue.
#[derive(Clone, Copy)]
struct AStarNode {
    pos: GridPos,
    f_score: f32,
}

impl PartialEq for AStarNode {
    fn eq(&self, other: &Self) -> bool {
        self.f_score == other.f_score
    }
}
impl Eq for AStarNode {}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap behavior
        other.f_score.partial_cmp(&self.f_score).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Heuristic: octile distance (allows diagonal movement).
fn heuristic(a: GridPos, b: GridPos) -> f32 {
    let dx = (a.0 as f32 - b.0 as f32).abs();
    let dy = (a.1 as f32 - b.1 as f32).abs();
    let diag = dx.min(dy);
    let straight = dx.max(dy) - diag;
    diag * 1.414 + straight
}

/// 8-directional neighbors.
const DIRS: [(i32, i32); 8] = [
    (-1, -1), (0, -1), (1, -1),
    (-1,  0),          (1,  0),
    (-1,  1), (0,  1), (1,  1),
];

/// Run A* on the nest grid. Returns path as list of grid positions (start excluded, goal included).
pub fn astar(grid: &NestGrid, start: GridPos, goal: GridPos) -> Option<Vec<GridPos>> {
    if !grid.get(start.0, start.1).is_passable() || !grid.get(goal.0, goal.1).is_passable() {
        return None;
    }
    if start == goal {
        return Some(vec![goal]);
    }

    let mut open = BinaryHeap::new();
    let mut g_scores: HashMap<GridPos, f32> = HashMap::new();
    let mut came_from: HashMap<GridPos, GridPos> = HashMap::new();

    g_scores.insert(start, 0.0);
    open.push(AStarNode {
        pos: start,
        f_score: heuristic(start, goal),
    });

    while let Some(current) = open.pop() {
        if current.pos == goal {
            // Reconstruct path
            let mut path = Vec::new();
            let mut node = goal;
            while node != start {
                path.push(node);
                node = came_from[&node];
            }
            path.reverse();
            return Some(path);
        }

        let current_g = g_scores[&current.pos];

        for &(dx, dy) in &DIRS {
            let nx = current.pos.0 as i32 + dx;
            let ny = current.pos.1 as i32 + dy;
            if nx < 0 || ny < 0 || nx as usize >= grid.width || ny as usize >= grid.height {
                continue;
            }
            let neighbor = (nx as usize, ny as usize);
            if !grid.get(neighbor.0, neighbor.1).is_passable() {
                continue;
            }

            // For diagonal moves, check that both adjacent cardinal cells are passable
            // (prevents cutting through wall corners)
            if dx != 0 && dy != 0 {
                let cx = (current.pos.0 as i32 + dx) as usize;
                let cy = current.pos.1;
                let rx = current.pos.0;
                let ry = (current.pos.1 as i32 + dy) as usize;
                if !grid.get(cx, cy).is_passable() || !grid.get(rx, ry).is_passable() {
                    continue;
                }
            }

            let move_cost = if dx != 0 && dy != 0 { 1.414 } else { 1.0 };
            let tentative_g = current_g + move_cost;

            if tentative_g < *g_scores.get(&neighbor).unwrap_or(&f32::MAX) {
                g_scores.insert(neighbor, tentative_g);
                came_from.insert(neighbor, current.pos);
                open.push(AStarNode {
                    pos: neighbor,
                    f_score: tentative_g + heuristic(neighbor, goal),
                });
            }
        }
    }

    None // No path found
}

/// Cached path entry.
struct CachedPath {
    path: Vec<GridPos>,
    generation: u32,
}

/// Path cache — avoids redundant A* queries.
/// Invalidated when nest grid changes (new tunnel dug).
#[derive(Component)]
pub struct NestPathCache {
    cache: HashMap<(GridPos, GridPos), CachedPath>,
    pub generation: u32,
}

impl Default for NestPathCache {
    fn default() -> Self {
        Self {
            cache: HashMap::new(),
            generation: 0,
        }
    }
}

impl NestPathCache {
    /// Get a cached path if it exists and is from the current generation.
    pub fn get(&self, start: GridPos, goal: GridPos) -> Option<&[GridPos]> {
        self.cache.get(&(start, goal)).and_then(|entry| {
            if entry.generation == self.generation {
                Some(entry.path.as_slice())
            } else {
                None
            }
        })
    }

    /// Store a path in the cache.
    pub fn insert(&mut self, start: GridPos, goal: GridPos, path: Vec<GridPos>) {
        self.cache.insert(
            (start, goal),
            CachedPath {
                path,
                generation: self.generation,
            },
        );
    }

    /// Invalidate all cached paths (call when nest grid changes).
    pub fn invalidate(&mut self) {
        self.generation += 1;
    }

    /// Find a path, using cache if available, otherwise compute via A*.
    pub fn find_path(&mut self, grid: &NestGrid, start: GridPos, goal: GridPos) -> Option<Vec<GridPos>> {
        if let Some(cached) = self.get(start, goal) {
            return Some(cached.to_vec());
        }

        if let Some(path) = astar(grid, start, goal) {
            self.insert(start, goal, path.clone());
            Some(path)
        } else {
            None
        }
    }
}
