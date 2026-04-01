use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellType {
    Soil,
    SoftSoil,
    Clay,
    Rock,
    Tunnel,
    Chamber(ChamberKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChamberKind {
    Queen,
    Brood,
    FoodStorage,
    Midden,
}

impl CellType {
    pub fn is_passable(&self) -> bool {
        matches!(self, CellType::Tunnel | CellType::Chamber(_))
    }

    pub fn color(&self) -> Color {
        match self {
            CellType::Soil => Color::srgb(0.45, 0.32, 0.18),
            CellType::SoftSoil => Color::srgb(0.50, 0.36, 0.20),
            CellType::Clay => Color::srgb(0.55, 0.40, 0.25),
            CellType::Rock => Color::srgb(0.4, 0.4, 0.4),
            CellType::Tunnel => Color::srgb(0.15, 0.10, 0.05),
            CellType::Chamber(kind) => match kind {
                ChamberKind::Queen => Color::srgb(0.25, 0.12, 0.18),
                ChamberKind::Brood => Color::srgb(0.22, 0.15, 0.10),
                ChamberKind::FoodStorage => Color::srgb(0.20, 0.18, 0.08),
                ChamberKind::Midden => Color::srgb(0.18, 0.16, 0.12),
            },
        }
    }
}

#[derive(Component)]
pub struct NestTile {
    pub grid_x: usize,
    pub grid_y: usize,
}

#[derive(Component)]
pub struct Queen;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BroodStage {
    Egg,
    Larva,
    Pupa,
}

#[derive(Component)]
pub struct Brood {
    pub stage: BroodStage,
    pub timer: f32,
    pub fed: bool,
}

impl Brood {
    pub fn new_egg() -> Self {
        Self {
            stage: BroodStage::Egg,
            timer: 0.0,
            fed: false,
        }
    }

    pub fn stage_duration(&self) -> f32 {
        match self.stage {
            BroodStage::Egg => 30.0,
            BroodStage::Larva => 45.0,
            BroodStage::Pupa => 30.0,
        }
    }
}
