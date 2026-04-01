use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PheromoneType {
    Home,
    Food,
    Alarm,
    Trail,
    Recruit,
}

impl PheromoneType {
    pub const COUNT: usize = 5;

    pub fn index(&self) -> usize {
        match self {
            PheromoneType::Home => 0,
            PheromoneType::Food => 1,
            PheromoneType::Alarm => 2,
            PheromoneType::Trail => 3,
            PheromoneType::Recruit => 4,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            PheromoneType::Home => Color::srgba(0.2, 0.4, 1.0, 0.7),
            PheromoneType::Food => Color::srgba(1.0, 0.6, 0.1, 0.7),
            PheromoneType::Alarm => Color::srgba(1.0, 0.2, 0.2, 0.7),
            PheromoneType::Trail => Color::srgba(1.0, 0.9, 0.1, 0.7),
            PheromoneType::Recruit => Color::srgba(0.3, 0.9, 1.0, 0.7),
        }
    }
}

/// Marker for pheromone overlay sprites
#[derive(Component)]
pub struct PheromoneOverlayTile {
    pub grid_x: usize,
    pub grid_y: usize,
}
