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

/// Convert an LDtk IntGrid value to a CellType.
/// Mapping: 1=Soil, 2=SoftSoil, 3=Clay, 4=Rock, 5=Tunnel,
/// 6=Chamber(Queen), 7=Chamber(Brood), 8=Chamber(FoodStorage), 9=Chamber(Midden)
pub fn intgrid_to_celltype(value: i32) -> CellType {
    match value {
        1 => CellType::Soil,
        2 => CellType::SoftSoil,
        3 => CellType::Clay,
        4 => CellType::Rock,
        5 => CellType::Tunnel,
        6 => CellType::Chamber(ChamberKind::Queen),
        7 => CellType::Chamber(ChamberKind::Brood),
        8 => CellType::Chamber(ChamberKind::FoodStorage),
        9 => CellType::Chamber(ChamberKind::Midden),
        _ => CellType::Soil,
    }
}

impl CellType {
    pub fn is_passable(&self) -> bool {
        matches!(self, CellType::Tunnel | CellType::Chamber(_))
    }

    /// Whether this cell can be excavated by a digger ant.
    pub fn is_diggable(&self) -> bool {
        matches!(self, CellType::Soil | CellType::SoftSoil | CellType::Clay)
    }

    /// Tileset tile index for this cell type in `nest.png`.
    /// Must match the tile order in `assets/tilesets/nest.png`.
    pub fn tile_index(&self) -> u32 {
        match self {
            CellType::Soil => 0,
            CellType::SoftSoil => 1,
            CellType::Clay => 2,
            CellType::Rock => 3,
            CellType::Tunnel => 4,
            CellType::Chamber(kind) => match kind {
                ChamberKind::Queen => 5,
                ChamberKind::Brood => 6,
                ChamberKind::FoodStorage => 7,
                ChamberKind::Midden => 8,
            },
        }
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
            CellType::Tunnel => Color::srgb(0.35, 0.35, 0.35),
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

/// Tracks how recently the queen has been fed.
/// Satiation fills when fed; each egg costs 0.2 satiation (5 eggs at full).
#[derive(Component)]
pub struct QueenHunger {
    /// Current satiation level (0.0 = starving, 1.0 = full).
    pub satiation: f32,
    /// Rate at which satiation decays per second (very slow; ants drive feeding).
    pub decay_rate: f32,
    /// Time spent at 0 satiation; used to trigger starvation damage after a grace period.
    pub starvation_timer: f32,
}

impl Default for QueenHunger {
    fn default() -> Self {
        Self {
            satiation: 1.0,       // start full so first eggs can happen immediately
            decay_rate: 0.005,    // ~200 seconds to go from 1.0 to 0.0
            starvation_timer: 0.0,
        }
    }
}

/// Queen behavioral state machine. Driven by utility scoring in queen_ai plugin.
#[derive(Component, Debug, Clone)]
pub enum QueenTask {
    /// Waiting to be assigned a task; timer tracks time since last evaluation.
    Idle { timer: f32 },
    /// Actively laying eggs at regular intervals.
    LayingEggs { egg_timer: f32 },
    /// Conserving energy — hunger decays at half rate.
    Resting { timer: f32 },
    /// Self-grooming — default moderate-priority activity.
    Grooming { timer: f32 },
}

/// Marks a brood entity as being physically carried by an ant.
#[derive(Component)]
pub struct CarriedBy(pub Entity);

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
    /// Whether this brood has been moved from the queen chamber to the brood chamber.
    pub relocated: bool,
}

impl Brood {
    pub fn new_egg() -> Self {
        Self {
            stage: BroodStage::Egg,
            timer: 0.0,
            fed: false,
            relocated: false,
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

#[derive(Component)]
pub struct FoodEntity {
    pub amount: f32,
}

impl FoodEntity {
    pub fn new(amount: f32) -> Self {
        Self { amount }
    }
}

#[derive(Component)]
pub struct StackedItem {
    pub grid_pos: (usize, usize),
    pub stack_index: u8,
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


/// Task assigned to a nest ant by the utility AI.
#[derive(Component, Debug, Clone)]
pub enum NestTask {
    FeedLarva { step: FeedStep, target_larva: Option<Entity> },
    MoveBrood { step: MoveBroodStep, target_brood: Option<Entity> },
    HaulFood { step: HaulStep },
    AttendQueen { step: AttendStep },
    Dig { step: DigStep, target_cell: Option<GridPos>, dig_timer: f32 },
    /// Ant patrols the nest, scanning for stimuli to respond to.
    Wander { scan_timer: f32, wander_time: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveBroodStep {
    GoToQueen,
    PickUpBrood,
    GoToBrood,
    PlaceBrood,
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
    GoToStorage,
    PickUpFood,
    GoToQueen,
    FeedQueen,
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
    /// Whether the ant with this task is currently carrying an item (food, soil, or brood).
    /// Used for tunnel traffic priority — laden ants get right-of-way.
    pub fn is_carrying(&self) -> bool {
        match self {
            NestTask::FeedLarva { step, .. } => matches!(
                step,
                FeedStep::GoToBrood | FeedStep::FindLarva | FeedStep::DeliverFood
            ),
            NestTask::MoveBrood { step, .. } => matches!(
                step,
                MoveBroodStep::GoToBrood | MoveBroodStep::PlaceBrood
            ),
            NestTask::HaulFood { step } => matches!(
                step,
                HaulStep::GoToStorage | HaulStep::DropFood
            ),
            NestTask::AttendQueen { step } => matches!(
                step,
                AttendStep::GoToQueen | AttendStep::FeedQueen
            ),
            NestTask::Dig { step, .. } => matches!(
                step,
                DigStep::GoToMidden | DigStep::DropSoil
            ),
            NestTask::Wander { .. } => false,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            NestTask::FeedLarva { .. } => "N",
            NestTask::MoveBrood { .. } => "M",
            NestTask::HaulFood { .. } => "H",
            NestTask::AttendQueen { .. } => "Q",
            NestTask::Dig { .. } => "D",
            NestTask::Wander { .. } => "W",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            NestTask::FeedLarva { .. } => Color::srgb(0.8, 0.6, 1.0),
            NestTask::MoveBrood { .. } => Color::srgb(1.0, 0.7, 0.8),
            NestTask::HaulFood { .. } => Color::srgb(0.6, 0.9, 0.3),
            NestTask::AttendQueen { .. } => Color::srgb(1.0, 0.8, 0.2),
            NestTask::Dig { .. } => Color::srgb(0.7, 0.5, 0.3),
            NestTask::Wander { .. } => Color::srgb(0.5, 0.6, 0.8),
        }
    }
}
