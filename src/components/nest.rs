use bevy::prelude::*;

use crate::resources::nest_pathfinding::GridPos;

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

    /// Whether this cell can be excavated by a digger ant.
    pub fn is_diggable(&self) -> bool {
        matches!(self, CellType::Soil | CellType::SoftSoil | CellType::Clay)
    }

    /// Excavation time in seconds for this soil type.
    pub fn dig_duration(&self) -> f32 {
        match self {
            CellType::SoftSoil => 1.0,
            CellType::Soil => 3.0,
            CellType::Clay => 6.0,
            _ => f32::MAX, // Rock and passable cells can't be dug
        }
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

/// Stores a computed path through the nest tunnel network.
/// Ants with this component follow waypoints toward a destination.
#[derive(Component)]
pub struct NestPath {
    pub waypoints: Vec<GridPos>,
    pub current_index: usize,
}

impl NestPath {
    pub fn new(waypoints: Vec<GridPos>) -> Self {
        Self {
            waypoints,
            current_index: 0,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.current_index >= self.waypoints.len()
    }

    pub fn destination(&self) -> Option<GridPos> {
        self.waypoints.last().copied()
    }
}

/// Marker for test ants that pathfind through the nest.
#[derive(Component)]
pub struct NestTestAnt {
    /// Chamber label index this ant is navigating toward.
    pub target_label: usize,
    /// Timer for picking a new destination after reaching current one.
    pub retarget_timer: f32,
}

/// Task assigned to a nest ant by the utility AI.
#[derive(Component, Debug, Clone)]
pub enum NestTask {
    FeedLarva { step: FeedStep, target_larva: Option<Entity> },
    HaulFood { step: HaulStep },
    AttendQueen { step: AttendStep },
    Dig { step: DigStep, target_cell: Option<GridPos>, dig_timer: f32 },
    Idle { timer: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedStep {
    GoToStorage,
    PickUpFood,
    GoToBrood,
    FindLarva,
    DeliverFood,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HaulStep {
    GoToEntrance,
    PickUpFood,
    GoToStorage,
    DropFood,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttendStep {
    GoToQueen,
    Grooming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigStep {
    GoToFace,
    Excavate,
    PickUpSoil,
    GoToMidden,
    DropSoil,
}

impl NestTask {
    pub fn label(&self) -> &'static str {
        match self {
            NestTask::FeedLarva { .. } => "N",
            NestTask::HaulFood { .. } => "H",
            NestTask::AttendQueen { .. } => "Q",
            NestTask::Dig { .. } => "D",
            NestTask::Idle { .. } => "I",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            NestTask::FeedLarva { .. } => Color::srgb(0.8, 0.6, 1.0),
            NestTask::HaulFood { .. } => Color::srgb(0.6, 0.9, 0.3),
            NestTask::AttendQueen { .. } => Color::srgb(1.0, 0.8, 0.2),
            NestTask::Dig { .. } => Color::srgb(0.7, 0.5, 0.3),
            NestTask::Idle { .. } => Color::srgba(1.0, 1.0, 1.0, 0.4),
        }
    }
}
