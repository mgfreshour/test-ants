use bevy::prelude::*;
use rand::Rng;
use crate::components::ant::{Ant, AntJob, AntState, CarriedItem, ColonyMember, Health, Movement, PlayerControlled, PositionHistory, SteeringTarget, SteeringWeights, TrailSense};
use crate::components::terrain::FoodSource;
use crate::resources::active_map::MapRegistry;
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};
use crate::resources::spatial_grid::SpatialGrid;

pub mod defending;
pub mod foraging;
pub mod hunger;
pub mod recruiting;
pub mod returning;
pub mod visuals;

pub use defending::ant_defender_patrol;
pub use foraging::ant_forage_steering;
pub use hunger::{hunger_tick, nest_ant_feeding, surface_ant_nest_feeding};
pub use recruiting::{ant_attack_recruit_steering, ant_follow_recruit_steering, nest_recruit_following};
pub use returning::ant_return_steering;
pub use visuals::{update_ant_visuals, spawn_state_labels, update_state_labels};

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
                    (
                        rebuild_spatial_grid,
                        hunger_tick,
                        surface_ant_nest_feeding,
                        nest_ant_feeding,
                        fix_orphaned_returners,
                        ant_forage_steering,
                        ant_defender_patrol,
                        ant_follow_recruit_steering,
                        ant_attack_recruit_steering,
                        nest_recruit_following,
                        ant_return_steering,
                    ).chain(),
                    (
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
                    ).chain(),
                ).chain(),
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
            AntJob::Unassigned,
            Movement::with_random_direction(config.ant_speed_worker, &mut rng),
            Health::worker(),
            ColonyMember { colony_id: 0 },
            PositionHistory::default(),
            TrailSense::default(),
            MapId(registry.surface),
            SteeringTarget::default(),
            SteeringWeights::default(),
        ));
    }
}

use crate::components::map::MapId;

fn rebuild_spatial_grid(
    mut grid: ResMut<SpatialGrid>,
    query: Query<(Entity, &Transform), With<Ant>>,
) {
    grid.clear();
    for (entity, transform) in &query {
        grid.insert(entity, transform.translation.truncate());
    }
}

fn fix_orphaned_returners(
    grids: Option<Res<crate::resources::pheromone::ColonyPheromones>>,
    mut query: Query<(&Transform, &ColonyMember, &mut Ant, &mut PositionHistory), (Without<CarriedItem>, Without<PlayerControlled>)>,
) {
    use crate::sim_core::regressions;
    use crate::components::pheromone::PheromoneType;

    for (transform, colony, mut ant, mut history) in &mut query {
        if regressions::should_reset_orphaned_returner(
            ant.state == AntState::Returning,
            false,
        ) {
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

const FOOD_PICKUP_RANGE: f32 = 20.0;

fn food_detection_and_pickup(
    clock: Res<SimClock>,
    mut commands: Commands,
    mut ant_query: Query<
        (Entity, &Transform, &mut Ant, &mut PositionHistory),
        Without<CarriedItem>,
    >,
    mut food_query: Query<(&Transform, &mut FoodSource)>,
) {
    use crate::sim_core::ant_logic;

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

fn nest_food_deposit(
    clock: Res<SimClock>,
    mut commands: Commands,
    registry: Res<MapRegistry>,
    portal_query: Query<&crate::components::map::MapPortal>,
    mut food_query: Query<&mut ColonyFood, With<crate::components::map::MapMarker>>,
    map_query: Query<&crate::resources::nest::NestGrid, With<crate::components::map::MapMarker>>,
    mut ant_query: Query<
        (Entity, &Transform, &mut Ant, &ColonyMember, &MapId, &CarriedItem, &mut PositionHistory),
    >,
) {
    use crate::sim_core::ant_logic;
    use crate::plugins::nest::nest_grid_to_world;

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
                            crate::components::nest::FoodEntity::new(1.0),
                            MapId(portal.target_map),
                        ));
                    }
                }
            }
            commands.entity(ant_entity).remove::<CarriedItem>();
            ant.hunger = ant_logic::apply_deposit_hunger_relief(ant.hunger, 0.3);
            ant.state = AntState::Foraging;
            history.clear();
        }
    }
}

fn ant_pheromone_deposit(
    clock: Res<SimClock>,
    pconfig: Res<crate::resources::pheromone::PheromoneConfig>,
    mut grids: Option<ResMut<crate::resources::pheromone::ColonyPheromones>>,
    query: Query<(&Transform, &Ant, &ColonyMember, Option<&CarriedItem>)>,
) {
    use crate::sim_core::ant_logic;
    use crate::components::pheromone::PheromoneType;

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
    use crate::sim_core::ant_logic;

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
            0.8,
            0.7,
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
    use crate::sim_core::ant_logic;

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
