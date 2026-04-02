use bevy::prelude::*;
use crate::components::ant::{Ant, CarriedItem, ColonyMember, Health};
use crate::components::map::{MapId, MapMarker};
use crate::components::nest::NestTask;
use crate::resources::active_map::MapRegistry;
use crate::resources::simulation::{SimClock, SimSpeed};
use crate::sim_core::ant_logic;
use super::ColonyFood;

const HUNGER_RATE: f32 = 0.004;
const HUNGER_SLOW_THRESHOLD: f32 = 0.8;
const HUNGER_SLOW_FACTOR: f32 = 0.7;
const STARVATION_DPS: f32 = 0.5;
const DEPOSIT_HUNGER_RELIEF: f32 = 0.3;
const NEST_FEED_RANGE: f32 = 60.0;
const NEST_FEED_THRESHOLD: f32 = 0.4;
const NEST_FEED_RELIEF: f32 = 0.5;
const NEST_FEED_COST: f32 = 0.2;

/// Increase hunger over time. Hungry ants slow down; starving ants take damage.
/// Ants carrying food can self-feed when hunger gets high enough.
pub fn hunger_tick(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut query: Query<(&mut Ant, &mut Health, Option<&CarriedItem>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();

    for (mut ant, mut health, carried) in &mut query {
        let (next_hunger, hp_loss) = ant_logic::hunger_tick_step(
            ant.hunger,
            dt,
            HUNGER_RATE,
            carried.map(|c| c.food_amount),
            0.5,
            0.4,
            STARVATION_DPS,
        );
        ant.hunger = next_hunger;
        health.current -= hp_loss;
    }
}

/// Surface ants near the nest portal eat from colony food stores when hungry.
pub fn surface_ant_nest_feeding(
    clock: Res<SimClock>,
    registry: Res<MapRegistry>,
    portal_query: Query<&crate::components::map::MapPortal>,
    mut food_query: Query<&mut ColonyFood, With<MapMarker>>,
    mut ant_query: Query<(&Transform, &ColonyMember, &MapId, &mut Ant), Without<CarriedItem>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    for (transform, colony, map_id, mut ant) in &mut ant_query {
        if map_id.0 != registry.surface || ant.hunger < 0.3 {
            continue;
        }

        let pos = transform.translation.truncate();

        let near_portal = portal_query.iter().any(|p| {
            p.map == registry.surface
                && p.colony_id.map_or(true, |id| id == colony.colony_id)
                && pos.distance(p.position) < NEST_FEED_RANGE
        });

        if !near_portal {
            continue;
        }

        // Find the nest's food store for this colony.
        let target_nest = portal_query.iter().find(|p| {
            p.map == registry.surface
                && p.colony_id.map_or(true, |id| id == colony.colony_id)
        });

        if let Some(portal) = target_nest {
            if let Ok(mut food) = food_query.get_mut(portal.target_map) {
                if food.stored > 0.5 {
                    food.stored -= 0.2;
                    ant.hunger = (ant.hunger - 0.5).max(0.0);
                }
            }
        }
    }
}

/// Nest ants eat from colony food stores when hungry.
pub fn nest_ant_feeding(
    clock: Res<SimClock>,
    mut map_query: Query<&mut ColonyFood, With<MapMarker>>,
    mut query: Query<(&mut Ant, &MapId), With<NestTask>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    for (mut ant, map_id) in &mut query {
        if ant.hunger < NEST_FEED_THRESHOLD {
            continue;
        }

        let Ok(mut colony_food) = map_query.get_mut(map_id.0) else { continue };
        if colony_food.stored < NEST_FEED_COST {
            continue;
        }

        colony_food.stored -= NEST_FEED_COST;
        ant.hunger = (ant.hunger - NEST_FEED_RELIEF).max(0.0);
    }
}
