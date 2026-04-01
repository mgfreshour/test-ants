use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{Ant, AntState, CarriedItem, ColonyMember, Follower, Health, Movement, PlayerControlled, PositionHistory, TrailSense};
use crate::components::map::{MapId, MapMarker, MapPortal};
use crate::components::nest::FoodEntity;

/// Hunger increases per second of simulation time. At this rate an ant goes
/// from 0 → 1.0 in ~250 seconds (~4 min) of sim time, giving foragers enough
/// time to locate distant food and return.
const HUNGER_RATE: f32 = 0.004;
/// Above this hunger threshold ants slow down.
const HUNGER_SLOW_THRESHOLD: f32 = 0.8;
/// Movement speed multiplier when very hungry.
const HUNGER_SLOW_FACTOR: f32 = 0.7;
/// HP lost per second when hunger is at 1.0 (starvation).
const STARVATION_DPS: f32 = 0.5;
/// Hunger reduction when a forager deposits food at the nest ("taste reward").
const DEPOSIT_HUNGER_RELIEF: f32 = 0.3;
use crate::components::pheromone::PheromoneType;
use crate::components::terrain::FoodSource;
use crate::resources::active_map::MapRegistry;
use crate::resources::nest::NestGrid;
use crate::resources::pheromone::{ColonyPheromones, PheromoneConfig};
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};
use crate::resources::spatial_grid::SpatialGrid;
use crate::sim_core::ant_logic;
use crate::plugins::nest::nest_grid_to_world;

pub struct AntAiPlugin;

#[derive(Component, Default)]
pub struct ColonyFood {
    pub stored: f32,
}

impl Plugin for AntAiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpatialGrid>()
            .add_systems(Startup, spawn_initial_ants)
            .add_systems(
                Update,
                (
                    rebuild_spatial_grid,
                    hunger_tick,
                    surface_ant_nest_feeding,
                    fix_orphaned_returners,
                    ant_forage_steering,
                    ant_return_steering,
                    food_detection_and_pickup,
                    nest_food_deposit,
                    ant_movement,
                    record_position_history,
                    ant_boundary_bounce,
                    ant_pheromone_deposit,
                    update_ant_visuals,
                    spawn_state_labels,
                    update_state_labels,
                    food_depletion_cleanup,
                )
                    .chain(),
            );
    }
}

fn spawn_initial_ants(mut commands: Commands, config: Res<SimConfig>, registry: Res<MapRegistry>) {
    let mut rng = rand::thread_rng();
    let nest = config.nest_position;

    for _ in 0..config.initial_ant_count {
        let offset_x = rng.gen_range(-20.0..20.0);
        let offset_y = rng.gen_range(-20.0..20.0);

        commands.spawn((
            Sprite {
                color: Color::srgb(0.1, 0.1, 0.1),
                custom_size: Some(Vec2::splat(4.0)),
                ..default()
            },
            Transform::from_xyz(nest.x + offset_x, nest.y + offset_y, 2.0),
            Ant::new_worker(),
            Movement::with_random_direction(config.ant_speed_worker, &mut rng),
            Health::worker(),
            ColonyMember { colony_id: 0 },
            PositionHistory::default(),
            TrailSense::default(),
            MapId(registry.surface),
        ));
    }
}

fn rebuild_spatial_grid(
    mut grid: ResMut<SpatialGrid>,
    query: Query<(Entity, &Transform), With<Ant>>,
) {
    grid.clear();
    for (entity, transform) in &query {
        grid.insert(entity, transform.translation.truncate());
    }
}

const ANTI_BACKTRACK_WEIGHT: f32 = 0.35;
const FORWARD_WEIGHT: f32 = 0.6;
const SENSE_RANGE: f32 = 60.0;
const PHERO_SENSE_RADIUS: i32 = 4;
const PHERO_TRAIL_WEIGHT: f32 = 1.5;
/// Fraction of ants that follow a detected trail; the rest scout independently.
/// Evaluated per-ant in stable ~3-second epochs so the decision doesn't flicker.
const TRAIL_FOLLOW_CHANCE: u32 = 60; // out of 100
const TRAIL_EPOCH_RATE: f32 = 0.33; // re-evaluate roughly every 3 seconds
/// Minimum local pheromone intensity for an ant to sense a trail at all.
/// Below this the concentration is too faint to follow, even if a gradient exists.
const MIN_SENSE_INTENSITY: f32 = 1.5;

/// Increase hunger over time. Hungry ants slow down; starving ants take damage.
/// Ants carrying food can self-feed when hunger gets high enough.
fn hunger_tick(
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
const NEST_FEED_RANGE: f32 = 60.0;

fn surface_ant_nest_feeding(
    clock: Res<SimClock>,
    registry: Res<MapRegistry>,
    portal_query: Query<&MapPortal>,
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

/// Reset ants stuck in invalid states back to Foraging.
fn fix_orphaned_returners(
    grids: Option<Res<ColonyPheromones>>,
    mut query: Query<(&Transform, &ColonyMember, &mut Ant, &mut PositionHistory), (Without<CarriedItem>, Without<PlayerControlled>)>,
) {
    for (transform, colony, mut ant, mut history) in &mut query {
        if ant.state == AntState::Returning {
            ant.state = AntState::Foraging;
            history.clear();
        }
        if ant.state == AntState::Defending {
            let should_reset = if let Some(ref all_grids) = grids {
                if let Some(grid) = all_grids.get(colony.colony_id) {
                    let pos = transform.translation.truncate();
                    if let Some((gx, gy)) = grid.world_to_grid(pos) {
                        grid.get(gx, gy, PheromoneType::Alarm) < 0.5
                    } else {
                        true
                    }
                } else {
                    true
                }
            } else {
                true
            };
            if should_reset {
                ant.state = AntState::Foraging;
                history.clear();
            }
        }
    }
}

/// Stable per-ant trail-follow decision. Returns true if this ant should follow
/// trails during the current epoch, false if it should scout instead.
fn should_follow_trail(entity: Entity, elapsed: f32) -> bool {
    ant_logic::should_follow_trail(entity.index(), elapsed, TRAIL_EPOCH_RATE, TRAIL_FOLLOW_CHANCE)
}

/// Foraging ants: follow FOOD pheromone gradient or random walk, biased away from HOME.
/// Within SENSE_RANGE of a food source, head straight for it.
fn ant_forage_steering(
    clock: Res<SimClock>,
    config: Res<SimConfig>,
    grids: Option<Res<ColonyPheromones>>,
    food_query: Query<&Transform, With<FoodSource>>,
    mut query: Query<(Entity, &Transform, &mut Movement, &Ant, &ColonyMember, &PositionHistory, &mut TrailSense), (Without<CarriedItem>, Without<PlayerControlled>, Without<Follower>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let mut rng = rand::thread_rng();
    let noise = config.exploration_noise;

    for (entity, transform, mut movement, ant, colony, history, mut sense) in &mut query {
        if ant.state != AntState::Foraging {
            continue;
        }

        let pos = transform.translation.truncate();
        let fwd = movement.direction;

        let mut nearest_food: Option<(f32, Vec2)> = None;
        for food_tf in &food_query {
            let food_pos = food_tf.translation.truncate();
            let dist = pos.distance(food_pos);
            if dist < SENSE_RANGE {
                if nearest_food.is_none() || dist < nearest_food.unwrap().0 {
                    nearest_food = Some((dist, food_pos));
                }
            }
        }

        if let Some((_, food_pos)) = nearest_food {
            let to_food = (food_pos - pos).normalize_or_zero();
            movement.direction = to_food;
            *sense = TrailSense::BeelineFood;
            continue;
        }

        let mut pheromone_bias = Vec2::ZERO;
        let mut on_trail = false;

        if let Some(ref all_grids) = grids {
            if let Some(grid) = all_grids.get(colony.colony_id) {
                if let Some((gx, gy)) = grid.world_to_grid(pos) {
                    let local = grid.get(gx, gy, PheromoneType::Food);
                    if local >= MIN_SENSE_INTENSITY
                        && should_follow_trail(entity, clock.elapsed)
                    {
                        let food_grad =
                            grid.sense_gradient(gx, gy, PheromoneType::Food, fwd, PHERO_SENSE_RADIUS);
                        if food_grad.length_squared() > 0.01 {
                            let fg = food_grad.normalize();
                            let along = fwd.dot(fg) * fwd;
                            let lateral = fg - along;
                            if lateral.length_squared() > 0.001 {
                                pheromone_bias += lateral.normalize() * PHERO_TRAIL_WEIGHT;
                            }
                            on_trail = true;
                        }
                    }
                }
            }
        }

        *sense = if on_trail { TrailSense::FollowingFood } else { TrailSense::Searching };

        let angle_offset = rng.gen_range(-noise..noise) * std::f32::consts::TAU;
        let current_angle = fwd.y.atan2(fwd.x);
        let new_angle = current_angle + angle_offset;
        let perturbed_fwd = Vec2::new(new_angle.cos(), new_angle.sin());

        let momentum = history.anti_backtrack(pos) * ANTI_BACKTRACK_WEIGHT;

        let noise_scale = if on_trail { 0.3 } else { 1.0 };
        let mut new_dir = (fwd * FORWARD_WEIGHT
            + perturbed_fwd * noise_scale
            + pheromone_bias
            + momentum)
            .normalize_or_zero();
        if new_dir == Vec2::ZERO {
            new_dir = perturbed_fwd;
        }
        movement.direction = new_dir;
    }
}

/// Returning ants: follow HOME pheromone gradient back to nest.
/// Within SENSE_RANGE of the nearest matching portal, head straight for it.
fn ant_return_steering(
    clock: Res<SimClock>,
    config: Res<SimConfig>,
    grids: Option<Res<ColonyPheromones>>,
    registry: Res<MapRegistry>,
    portal_query: Query<&MapPortal>,
    mut query: Query<(&Transform, &mut Movement, &Ant, &ColonyMember, &MapId, &PositionHistory, &mut TrailSense), (With<CarriedItem>, Without<PlayerControlled>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let mut rng = rand::thread_rng();
    let noise = config.exploration_noise * 0.5;

    for (transform, mut movement, ant, colony, map_id, history, mut sense) in &mut query {
        if ant.state != AntState::Returning {
            continue;
        }

        let pos = transform.translation.truncate();

        // Find the nearest portal on this ant's map leading toward a nest.
        let nest_pos = portal_query
            .iter()
            .filter(|p| {
                p.map == map_id.0
                    && p.target_map != registry.surface
                    && p.colony_id.map_or(true, |id| id == colony.colony_id)
            })
            .min_by(|a, b| {
                pos.distance(a.position)
                    .partial_cmp(&pos.distance(b.position))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|p| p.position)
            .unwrap_or(config.nest_position);
        let to_nest = nest_pos - pos;
        let dist_to_nest = to_nest.length();

        if dist_to_nest < SENSE_RANGE {
            movement.direction = to_nest.normalize_or_zero();
            *sense = TrailSense::BeelineNest;
            continue;
        }

        let fwd = movement.direction;
        let mut bias = Vec2::ZERO;

        let mut on_trail = false;

        if let Some(ref all_grids) = grids {
            if let Some(grid) = all_grids.get(colony.colony_id) {
                if let Some((gx, gy)) = grid.world_to_grid(pos) {
                    let local = grid.get(gx, gy, PheromoneType::Home);
                    if local >= MIN_SENSE_INTENSITY {
                        let home_grad =
                            grid.sense_gradient(gx, gy, PheromoneType::Home, fwd, PHERO_SENSE_RADIUS);
                        if home_grad.length_squared() > 0.01 {
                            bias += home_grad.normalize() * PHERO_TRAIL_WEIGHT;
                            on_trail = true;
                        }
                    }
                }
            }
        }

        *sense = if on_trail { TrailSense::FollowingHome } else { TrailSense::Searching };

        if dist_to_nest > 1.0 {
            bias += to_nest.normalize() * 0.03;
        }

        let angle_offset = rng.gen_range(-noise..noise) * std::f32::consts::TAU;
        let current_angle = fwd.y.atan2(fwd.x);
        let new_angle = current_angle + angle_offset;
        let perturbed_fwd = Vec2::new(new_angle.cos(), new_angle.sin());

        let momentum = history.anti_backtrack(pos) * ANTI_BACKTRACK_WEIGHT;

        let noise_scale = if on_trail { 0.3 } else { 1.0 };
        let mut new_dir = (fwd * FORWARD_WEIGHT + perturbed_fwd * noise_scale + bias + momentum)
            .normalize_or_zero();
        if new_dir == Vec2::ZERO {
            new_dir = perturbed_fwd;
        }
        movement.direction = new_dir;
    }
}

const FOOD_PICKUP_RANGE: f32 = 20.0;

/// Foraging ants near a food source pick it up
fn food_detection_and_pickup(
    clock: Res<SimClock>,
    mut commands: Commands,
    mut ant_query: Query<
        (Entity, &Transform, &mut Ant, &mut PositionHistory),
        Without<CarriedItem>,
    >,
    mut food_query: Query<(&Transform, &mut FoodSource)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    for (ant_entity, ant_transform, mut ant, mut history) in &mut ant_query {
        let ant_pos = ant_transform.translation.truncate();

        for (food_transform, mut food) in &mut food_query {
            let food_pos = food_transform.translation.truncate();
            let dist = ant_pos.distance(food_pos);

            if ant_logic::can_pickup_food(ant.state == AntState::Foraging, dist, FOOD_PICKUP_RANGE) {
                let Some(amount) = ant_logic::pickup_food_amount(food.remaining, 5.0) else {
                    continue;
                };
                food.remaining -= amount;
                commands.entity(ant_entity).insert(CarriedItem { food_amount: amount });
                ant.state = AntState::Returning;
                history.clear();
                break;
            }
        }
    }
}

const NEST_DEPOSIT_RANGE: f32 = 30.0;

/// Returning ants near a portal deposit food into the portal's target nest.
fn nest_food_deposit(
    clock: Res<SimClock>,
    mut commands: Commands,
    registry: Res<MapRegistry>,
    portal_query: Query<&MapPortal>,
    mut food_query: Query<&mut ColonyFood, With<MapMarker>>,
    map_query: Query<&NestGrid, With<MapMarker>>,
    mut ant_query: Query<
        (Entity, &Transform, &mut Ant, &ColonyMember, &MapId, &CarriedItem, &mut PositionHistory),
    >,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    for (ant_entity, ant_transform, mut ant, colony, map_id, carried, mut history) in &mut ant_query {
        if ant.state != AntState::Returning {
            continue;
        }

        // Only surface ants deposit food at portals.
        let is_surface_ant = map_id.0 == registry.surface;
        if !is_surface_ant {
            continue;
        }

        let ant_pos = ant_transform.translation.truncate();

        // Find a portal on the surface leading to a nest that matches this colony.
        let deposit_target = portal_query.iter().find(|p| {
            p.map == registry.surface
                && p.colony_id.map_or(true, |id| id == colony.colony_id)
                && ant_logic::can_deposit_food(
                    ant.state == AntState::Returning,
                    is_surface_ant,
                    ant_pos.distance(p.position),
                    NEST_DEPOSIT_RANGE,
                )
        });

        if let Some(portal) = deposit_target {
            if let Ok(mut food) = food_query.get_mut(portal.target_map) {
                food.stored += carried.food_amount;

                // Spawn physical food entities at entrance
                if let Ok(grid) = map_query.get(portal.target_map) {
                    let mut rng = rand::thread_rng();
                    let num_entities = carried.food_amount.floor() as usize;
                    let cx = grid.width / 2;
                    for i in 0..num_entities {
                        let gy = (i % 7).min(6);  // spread across entrance tunnel
                        let world_pos = nest_grid_to_world(cx, gy);
                        let jitter = Vec2::new(
                            rng.gen_range(-3.0..3.0),
                            rng.gen_range(-3.0..3.0),
                        );

                        commands.spawn((
                            Sprite {
                                color: Color::srgb(0.9, 0.7, 0.2),
                                custom_size: Some(Vec2::splat(5.0)),
                                ..default()
                            },
                            Transform::from_xyz(world_pos.x + jitter.x, world_pos.y + jitter.y, 2.5),
                            Visibility::Hidden,
                            FoodEntity::new(1.0),
                            MapId(portal.target_map),
                        ));
                    }
                }
            }
            commands.entity(ant_entity).remove::<CarriedItem>();
            ant.hunger = ant_logic::apply_deposit_hunger_relief(ant.hunger, DEPOSIT_HUNGER_RELIEF);
            ant.state = AntState::Foraging;
            history.clear();
        }
    }
}

/// Ants deposit pheromones based on state — into their own colony's grid
fn ant_pheromone_deposit(
    clock: Res<SimClock>,
    pconfig: Res<PheromoneConfig>,
    mut grids: Option<ResMut<ColonyPheromones>>,
    query: Query<(&Transform, &Ant, &ColonyMember, Option<&CarriedItem>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let Some(ref mut all_grids) = grids else { return };

    for (transform, ant, colony, carried) in &query {
        let Some(grid) = all_grids.get_mut(colony.colony_id) else {
            continue;
        };
        let pos = transform.translation.truncate();
        let Some((gx, gy)) = grid.world_to_grid(pos) else {
            continue;
        };

        match ant.state {
            AntState::Foraging => {
                let amt = pconfig.deposit_amount(PheromoneType::Home);
                if let Some(dep) =
                    ant_logic::home_pheromone_deposit_amount(ant.state == AntState::Foraging, amt)
                {
                    grid.deposit(gx, gy, PheromoneType::Home, dep, pconfig.max_intensity);
                }
            }
            AntState::Returning => {
                let base = pconfig.deposit_amount(PheromoneType::Food);
                if let Some(dep) = ant_logic::food_pheromone_deposit_amount(
                    ant.state == AntState::Returning,
                    base,
                    carried.map(|c| c.food_amount),
                ) {
                    grid.deposit(gx, gy, PheromoneType::Food, dep, pconfig.max_intensity);
                }
            }
            _ => {}
        }
    }
}

fn ant_movement(
    clock: Res<SimClock>,
    time: Res<Time>,
    registry: Res<MapRegistry>,
    mut query: Query<(&mut Transform, &Movement, &Ant, &MapId), Without<PlayerControlled>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();

    for (mut transform, movement, ant, map_id) in &mut query {
        // Surface movement only applies to surface ants.
        if map_id.0 != registry.surface {
            continue;
        }
        let velocity = ant_logic::surface_velocity(
            movement.direction,
            movement.speed,
            ant.hunger,
            dt,
            HUNGER_SLOW_THRESHOLD,
            HUNGER_SLOW_FACTOR,
        );
        transform.translation.x += velocity.x;
        transform.translation.y += velocity.y;
    }
}

fn record_position_history(
    clock: Res<SimClock>,
    mut query: Query<(&Transform, &mut PositionHistory), With<Ant>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    for (transform, mut history) in &mut query {
        history.record(transform.translation.truncate());
    }
}

fn ant_boundary_bounce(
    clock: Res<SimClock>,
    config: Res<SimConfig>,
    registry: Res<MapRegistry>,
    mut query: Query<(&mut Transform, &mut Movement, &MapId), With<Ant>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let margin = 8.0;
    let min_x = margin;
    let max_x = config.world_width - margin;
    let min_y = margin;
    let max_y = config.world_height - margin;

    for (mut transform, mut movement, map_id) in &mut query {
        // Boundary bounce only applies to surface ants.
        if map_id.0 != registry.surface {
            continue;
        }
        let (next_pos, next_dir) = ant_logic::apply_boundary_bounce(
            transform.translation.truncate(),
            movement.direction,
            Vec2::new(min_x, min_y),
            Vec2::new(max_x, max_y),
        );
        transform.translation.x = next_pos.x;
        transform.translation.y = next_pos.y;
        movement.direction = next_dir;
    }
}

/// Tint ants based on state: dark = foraging, green-tinted = carrying food
fn update_ant_visuals(
    mut query: Query<(&Ant, &ColonyMember, &mut Sprite, Option<&CarriedItem>), Without<PlayerControlled>>,
) {
    for (ant, colony, mut sprite, carried) in &mut query {
        let is_red = colony.colony_id != 0;
        let fighting = ant.state == AntState::Defending || ant.state == AntState::Fighting;

        sprite.color = match (is_red, carried.is_some(), fighting) {
            (_, _, true) => Color::srgb(1.0, 0.2, 0.2),
            (true, true, _) => Color::srgb(0.9, 0.3, 0.1),
            (true, false, _) => Color::srgb(0.7, 0.15, 0.1),
            (false, true, _) => Color::srgb(0.9, 0.4, 0.1),
            (false, false, _) => Color::srgb(0.1, 0.1, 0.1),
        };
    }
}

/// Despawn fully depleted food sources
fn food_depletion_cleanup(
    mut commands: Commands,
    query: Query<(Entity, &FoodSource)>,
) {
    for (entity, food) in &query {
        if food.remaining <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component)]
struct StateLabel;

fn spawn_state_labels(
    mut commands: Commands,
    query: Query<Entity, (With<Ant>, Without<Children>)>,
) {
    for entity in &query {
        let child = commands.spawn((
            Text2d::new("F"),
            TextFont {
                font_size: 8.0,
                ..default()
            },
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
            Transform::from_xyz(0.0, 5.0, 0.1),
            StateLabel,
        )).id();
        commands.entity(entity).add_child(child);
    }
}

fn update_state_labels(
    ant_query: Query<(&Ant, Option<&TrailSense>, &Children)>,
    mut label_query: Query<(&mut Text2d, &mut TextColor), With<StateLabel>>,
) {
    for (ant, sense, children) in &ant_query {
        let sense = sense.copied().unwrap_or_default();

        let (letter, color) = match ant.state {
            AntState::Defending | AntState::Fighting => ("!", Color::srgb(1.0, 0.3, 0.3)),
            AntState::Following => (">", Color::srgb(0.5, 0.8, 1.0)),
            AntState::Idle => ("I", Color::srgba(1.0, 1.0, 1.0, 0.5)),
            AntState::Nursing => ("N", Color::srgb(0.8, 0.6, 1.0)),
            AntState::Digging => ("G", Color::srgb(0.7, 0.5, 0.3)),
            AntState::Fleeing => ("X", Color::srgb(1.0, 1.0, 0.2)),
            AntState::Foraging => match sense {
                TrailSense::FollowingFood => ("f", Color::srgb(1.0, 0.6, 0.1)),
                TrailSense::FollowingAlarm => ("a", Color::srgb(1.0, 0.2, 0.2)),
                TrailSense::FollowingTrail => ("t", Color::srgb(0.8, 0.8, 0.2)),
                TrailSense::BeelineFood => ("*", Color::srgb(0.2, 1.0, 0.2)),
                _ => ("?", Color::srgba(1.0, 1.0, 1.0, 0.6)),
            },
            AntState::Returning => match sense {
                TrailSense::FollowingHome => ("h", Color::srgb(0.4, 0.6, 1.0)),
                TrailSense::BeelineNest => ("^", Color::srgb(0.3, 1.0, 1.0)),
                _ => ("r", Color::srgba(1.0, 1.0, 1.0, 0.6)),
            },
        };

        for &child in children.iter() {
            if let Ok((mut text, mut text_color)) = label_query.get_mut(child) {
                **text = letter.to_string();
                *text_color = TextColor(color);
            }
        }
    }
}
