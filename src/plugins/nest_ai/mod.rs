// Nest underground AI systems: task execution, pathfinding, pheromones, excavation
// This module coordinates underground ant behavior, including utility scoring,
// task transitions, job assignment, and excavation logic.

pub mod core;

pub use core::{NestAiPlugin, apply_flood_damage, apply_brood_fed, apply_brood_relocated};
