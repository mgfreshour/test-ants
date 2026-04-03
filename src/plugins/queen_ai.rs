use bevy::prelude::*;

use crate::components::ant::{ColonyMember, Health};
use crate::components::map::{MapId, MapMarker};
use crate::components::nest::{Brood, ChamberKind, CellType, Queen, QueenHunger, QueenTask};
use crate::plugins::ant_ai::ColonyFood;
use crate::resources::nest::NestGrid;
use crate::resources::simulation::{SimClock, SimSpeed};
use crate::sim_core::queen_scoring::{
    self, QueenScoringInput, QueenTaskChoice,
};

pub struct QueenAiPlugin;

/// How long a queen sits in Idle before re-evaluating tasks.
const REEVALUATE_INTERVAL: f32 = 2.0;
/// How long a resting bout lasts before returning to idle for re-evaluation.
const REST_DURATION: f32 = 10.0;
/// How long a grooming bout lasts before returning to idle for re-evaluation.
const GROOM_DURATION: f32 = 8.0;

impl Plugin for QueenAiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                advance_queen_idle,
                queen_utility_scoring,
                advance_queen_resting,
                advance_queen_grooming,
            ),
        );
    }
}

/// Tick the idle timer so re-evaluation can trigger.
fn advance_queen_idle(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut queen_query: Query<&mut QueenTask, With<Queen>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }
    let dt = time.delta_secs() * clock.speed.multiplier();

    for mut task in &mut queen_query {
        if let QueenTask::Idle { ref mut timer } = *task {
            *timer += dt;
        }
    }
}

/// Evaluate queen utility scores and assign a new task when idle long enough.
#[allow(clippy::too_many_arguments)]
fn queen_utility_scoring(
    mut queen_query: Query<(&QueenHunger, &Health, &MapId, &ColonyMember, &mut QueenTask), With<Queen>>,
    grid_query: Query<&NestGrid, With<MapMarker>>,
    brood_query: Query<&Brood>,
    food_query: Query<&ColonyFood, With<MapMarker>>,
) {
    for (hunger, health, map_id, colony, mut task) in &mut queen_query {
        let QueenTask::Idle { timer } = *task else { continue };
        if timer < REEVALUATE_INTERVAL {
            continue;
        }

        // Gather scoring input
        let brood_count = brood_query.iter().count() as u32;
        let colony_food_stored = food_query.iter().find(|_| true).map_or(0.0, |f| f.stored);
        let has_queen_chamber = grid_query.get(map_id.0).ok().map_or(false, |grid| {
            (0..grid.height).any(|y| (0..grid.width).any(|x| grid.get(x, y) == CellType::Chamber(ChamberKind::Queen)))
        });

        let input = QueenScoringInput {
            satiation: hunger.satiation,
            health_frac: (health.current / health.max).clamp(0.0, 1.0),
            brood_count,
            colony_food_stored,
            has_queen_chamber,
        };

        let choice = queen_scoring::choose_queen_task(&queen_scoring::compute_queen_scores(&input));

        *task = match choice {
            QueenTaskChoice::LayingEggs => QueenTask::LayingEggs { egg_timer: 0.0 },
            QueenTaskChoice::Resting => QueenTask::Resting { timer: 0.0 },
            QueenTaskChoice::Grooming => QueenTask::Grooming { timer: 0.0 },
            QueenTaskChoice::Idle => QueenTask::Idle { timer: 0.0 },
        };
    }
}

/// Resting bout — transitions back to idle after REST_DURATION.
fn advance_queen_resting(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut queen_query: Query<&mut QueenTask, With<Queen>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }
    let dt = time.delta_secs() * clock.speed.multiplier();

    for mut task in &mut queen_query {
        if let QueenTask::Resting { ref mut timer } = *task {
            *timer += dt;
            if *timer >= REST_DURATION {
                *task = QueenTask::Idle { timer: 0.0 };
            }
        }
    }
}

/// Grooming bout — transitions back to idle after GROOM_DURATION.
fn advance_queen_grooming(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut queen_query: Query<&mut QueenTask, With<Queen>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }
    let dt = time.delta_secs() * clock.speed.multiplier();

    for mut task in &mut queen_query {
        if let QueenTask::Grooming { ref mut timer } = *task {
            *timer += dt;
            if *timer >= GROOM_DURATION {
                *task = QueenTask::Idle { timer: 0.0 };
            }
        }
    }
}
