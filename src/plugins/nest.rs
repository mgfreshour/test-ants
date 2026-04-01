use bevy::prelude::*;
use rand::Rng;

use crate::components::ant::{Ant, Caste, ColonyMember, Health, Movement, PositionHistory, TrailSense};
use crate::components::map::{MapId, MapKind, MapMarker, spawn_portal_pair};
use crate::components::nest::{Brood, BroodStage, CellType, ChamberKind, NestTile, Queen, QueenHunger};
use crate::plugins::ant_ai::ColonyFood;
use crate::plugins::camera::MainCamera;
use crate::resources::active_map::{ActiveMap, MapRegistry, SavedCamera, SavedCameraStates};
use crate::resources::colony::{BehaviorSliders, CasteRatios, ColonyStats};
use crate::resources::nest::{NestGrid, PlayerDigZones, NEST_CELL_SIZE, NEST_HEIGHT, NEST_WIDTH};
use crate::resources::nest_pathfinding::NestPathCache;
use crate::resources::nest_pheromone::NestPheromoneGrid;
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};

pub struct NestPlugin;


const QUEEN_EGG_INTERVAL: f32 = 10.0;
/// Satiation consumed per egg (5 eggs from a full queen).
const EGG_SATIATION_COST: f32 = 0.2;
/// Grace period at 0 satiation before starvation damage starts.
const STARVATION_GRACE_PERIOD: f32 = 30.0;
/// Health lost per second after grace period expires.
const STARVATION_DAMAGE_RATE: f32 = 0.5;

/// Default nest-view camera scale.
const NEST_CAMERA_SCALE: f32 = 0.7;

pub fn nest_grid_to_world(gx: usize, gy: usize) -> Vec2 {
    let offset_x = -(NEST_WIDTH as f32 * NEST_CELL_SIZE) / 2.0;
    let offset_y = (NEST_HEIGHT as f32 * NEST_CELL_SIZE) / 2.0;
    Vec2::new(
        offset_x + gx as f32 * NEST_CELL_SIZE + NEST_CELL_SIZE / 2.0,
        offset_y - gy as f32 * NEST_CELL_SIZE - NEST_CELL_SIZE / 2.0,
    )
}

impl Plugin for NestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ColonyStats>()
            .init_resource::<CasteRatios>()
            // Map entities created in PreStartup so Startup systems can query them.
            .add_systems(PreStartup, setup_maps)
            .add_systems(
                Startup,
                (render_nest, spawn_queen).after(setup_maps),
            )
            .add_systems(
                Update,
                (
                    cycle_map_view,
                    sync_map_visibility,
                    queen_hunger_decay,
                    queen_starvation_damage.after(queen_hunger_decay),
                    queen_egg_laying,
                    brood_development,
                    update_colony_stats,
                ),
            );
    }
}

// ── Map entity setup ──────────────────────────────────────────────────

/// Spawns surface and player-nest map entities, portals between them,
/// and inserts `ActiveMap`, `MapRegistry`, and `SavedCameraStates`.
/// Runs in PreStartup so all Startup systems can query the map entities.
fn setup_maps(mut commands: Commands, config: Res<SimConfig>) {
    // Surface map — no grid/pheromone components needed.
    let surface = commands.spawn((MapMarker, MapKind::Surface)).id();

    // Player nest map — carries all per-nest data.
    let nest_grid = NestGrid::default();
    let mut phero_grid = NestPheromoneGrid::default();
    phero_grid.seed_from_grid(&nest_grid);

    let player_nest = commands.spawn((
        MapMarker,
        MapKind::Nest { colony_id: 0 },
        nest_grid,
        phero_grid,
        NestPathCache::default(),
        ColonyFood::default(),
        BehaviorSliders::default(),
        PlayerDigZones::default(),
        crate::resources::nest::TileStackRegistry::default(),
    )).id();

    // Portal pair: surface nest entrance ↔ underground entrance cell.
    // Surface position = SimConfig.nest_position.
    // Underground position = top of the entrance tunnel (column cx, first passable row).
    let cx = NEST_WIDTH / 2;
    let underground_entrance = nest_grid_to_world(cx, 0);
    spawn_portal_pair(
        &mut commands,
        surface,
        config.nest_position,
        player_nest,
        underground_entrance,
        Some(0), // only player colony (id=0)
    );

    commands.insert_resource(ActiveMap { entity: surface, kind: MapKind::Surface });
    commands.insert_resource(MapRegistry {
        surface,
        player_nest,
        maps: vec![surface, player_nest],
    });
    commands.insert_resource(SavedCameraStates::default());
}

// ── Startup rendering ────────────────────────────────────────────────

fn render_nest(
    mut commands: Commands,
    registry: Res<MapRegistry>,
    map_query: Query<&NestGrid, With<MapMarker>>,
) {
    let Ok(grid) = map_query.get(registry.player_nest) else { return };

    for y in 0..grid.height {
        for x in 0..grid.width {
            let cell = grid.get(x, y);
            let w = nest_grid_to_world(x, y);

            commands.spawn((
                Sprite {
                    color: cell.color(),
                    custom_size: Some(Vec2::splat(NEST_CELL_SIZE)),
                    ..default()
                },
                Transform::from_xyz(w.x, w.y, 0.0),
                Visibility::Hidden,
                NestTile { grid_x: x, grid_y: y },
                MapId(registry.player_nest),
            ));
        }
    }
}

fn spawn_queen(mut commands: Commands, registry: Res<MapRegistry>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>) {
    let cx = NEST_WIDTH / 2;
    let queen_center = nest_grid_to_world(cx, 16);

    commands.spawn((
        Mesh2d(meshes.add(Circle::new(6.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(0.8, 0.6, 0.1)))),
        Transform::from_xyz(queen_center.x, queen_center.y, 3.0),
        Visibility::Hidden,
        Queen,
        QueenHunger::default(),
        MapId(registry.player_nest),
        Health { current: 100.0, max: 100.0 },
    ));
}

// ── View cycling ─────────────────────────────────────────────────────

/// Tab cycles through all maps in `MapRegistry.maps`, saving/restoring camera.
fn cycle_map_view(
    input: Res<ButtonInput<KeyCode>>,
    mut active: ResMut<ActiveMap>,
    registry: Res<MapRegistry>,
    map_kind_query: Query<&MapKind, With<MapMarker>>,
    mut saved: ResMut<SavedCameraStates>,
    mut camera_query: Query<(&mut Transform, &mut OrthographicProjection), With<MainCamera>>,
    config: Res<SimConfig>,
) {
    if !input.just_pressed(KeyCode::Tab) {
        return;
    }
    let Ok((mut cam_tf, mut proj)) = camera_query.get_single_mut() else { return };

    // Save current camera state for outgoing map.
    saved.0.insert(active.entity, SavedCamera {
        position: cam_tf.translation.truncate(),
        scale: proj.scale,
    });

    // Advance to next map.
    let current_idx = registry.maps.iter().position(|&m| m == active.entity).unwrap_or(0);
    let next_idx = (current_idx + 1) % registry.maps.len();
    let next_entity = registry.maps[next_idx];
    let next_kind = map_kind_query.get(next_entity).copied().unwrap_or(MapKind::Surface);

    // Restore camera for incoming map, or use defaults.
    if let Some(cam) = saved.0.get(&next_entity) {
        cam_tf.translation.x = cam.position.x;
        cam_tf.translation.y = cam.position.y;
        proj.scale = cam.scale;
    } else {
        match next_kind {
            MapKind::Surface => {
                cam_tf.translation.x = config.world_width / 2.0;
                cam_tf.translation.y = config.world_height / 2.0;
                proj.scale = 1.0;
            }
            MapKind::Nest { .. } | MapKind::SpecialZone { .. } => {
                cam_tf.translation.x = 0.0;
                cam_tf.translation.y = 0.0;
                proj.scale = NEST_CAMERA_SCALE;
            }
        }
    }

    active.entity = next_entity;
    active.kind = next_kind;
}

/// Show entities whose MapId matches the active map; hide all others.
/// Runs every frame but early-exits when nothing changed.
fn sync_map_visibility(
    active: Res<ActiveMap>,
    mut query: Query<(Ref<MapId>, &mut Visibility)>,
) {
    let map_changed = active.is_changed();
    for (map_id, mut vis) in &mut query {
        if !map_changed && !map_id.is_added() {
            continue;
        }
        *vis = if map_id.0 == active.entity {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

// ── Nest simulation ──────────────────────────────────────────────────

fn queen_hunger_decay(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut queen_query: Query<&mut QueenHunger, With<Queen>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }
    let dt = time.delta_secs() * clock.speed.multiplier();
    for mut hunger in &mut queen_query {
        hunger.satiation = (hunger.satiation - hunger.decay_rate * dt).max(0.0);
    }
}

fn queen_egg_laying(
    clock: Res<SimClock>,
    time: Res<Time>,
    registry: Res<MapRegistry>,
    mut commands: Commands,
    map_grid_query: Query<&NestGrid, With<MapMarker>>,
    mut queen_query: Query<&mut QueenHunger, With<Queen>>,
    mut egg_timer: Local<f32>,
) {
    let Ok(mut hunger) = queen_query.get_single_mut() else { return };
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let Ok(grid) = map_grid_query.get(registry.player_nest) else { return };

    let dt = time.delta_secs() * clock.speed.multiplier();
    *egg_timer += dt;

    // Queen lays an egg when the timer fires and she has enough satiation reserves.
    if *egg_timer >= QUEEN_EGG_INTERVAL && hunger.satiation >= EGG_SATIATION_COST {
        *egg_timer -= QUEEN_EGG_INTERVAL;
        hunger.satiation -= EGG_SATIATION_COST;

        let mut rng = rand::thread_rng();
        let queen_cells = find_chamber_cells(grid, ChamberKind::Queen);
        if queen_cells.is_empty() {
            return;
        }
        let &(gx, gy) = &queen_cells[rng.gen_range(0..queen_cells.len())];
        let pos = nest_grid_to_world(gx, gy);
        let jitter = Vec2::new(
            rng.gen_range(-NEST_CELL_SIZE * 0.35..NEST_CELL_SIZE * 0.35),
            rng.gen_range(-NEST_CELL_SIZE * 0.35..NEST_CELL_SIZE * 0.35),
        );

        // Eggs always start hidden; sync_map_visibility will show them if nest is active.
        commands.spawn((
            Sprite {
                color: Color::srgb(0.95, 0.95, 0.85),
                custom_size: Some(Vec2::splat(3.0)),
                ..default()
            },
            Transform::from_xyz(pos.x + jitter.x, pos.y + jitter.y, 2.5),
            Visibility::Hidden,
            Brood::new_egg(),
            MapId(registry.player_nest),
        ));
    }
}

fn queen_starvation_damage(
    clock: Res<SimClock>,
    time: Res<Time>,
    mut queen_query: Query<(&mut QueenHunger, &mut Health), With<Queen>>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }
    let dt = time.delta_secs() * clock.speed.multiplier();

    for (mut hunger, mut health) in &mut queen_query {
        if hunger.satiation <= 0.0 {
            hunger.starvation_timer += dt;
            if hunger.starvation_timer >= STARVATION_GRACE_PERIOD {
                health.current = (health.current - STARVATION_DAMAGE_RATE * dt).max(0.0);
            }
        } else {
            hunger.starvation_timer = 0.0;
        }
    }
}

fn brood_development(
    clock: Res<SimClock>,
    time: Res<Time>,
    config: Res<SimConfig>,
    caste_ratios: Res<CasteRatios>,
    registry: Res<MapRegistry>,
    mut commands: Commands,
    mut stack_query: Query<&mut crate::resources::nest::TileStackRegistry, With<MapMarker>>,
    mut brood_query: Query<(Entity, &mut Brood, &mut Sprite, Option<&crate::components::nest::StackedItem>)>,
) {
    if clock.speed == SimSpeed::Paused {
        return;
    }

    let dt = time.delta_secs() * clock.speed.multiplier();
    let mut rng = rand::thread_rng();

    for (entity, mut brood, mut sprite, stacked_opt) in &mut brood_query {
        brood.timer += dt;

        if brood.timer >= brood.stage_duration() {
            brood.timer = 0.0;
            match brood.stage {
                BroodStage::Egg => {
                    brood.stage = BroodStage::Larva;
                    sprite.color = Color::srgb(0.9, 0.85, 0.7);
                    sprite.custom_size = Some(Vec2::splat(4.0));
                }
                BroodStage::Larva => {
                    brood.stage = BroodStage::Pupa;
                    sprite.color = Color::srgb(0.7, 0.65, 0.5);
                    sprite.custom_size = Some(Vec2::splat(5.0));
                }
                BroodStage::Pupa => {
                    // Remove from stack registry
                    if let Ok(mut stack_reg) = stack_query.get_mut(registry.player_nest) {
                        if let Some(stacked) = stacked_opt {
                            stack_reg.remove(stacked.grid_pos, entity);
                        }
                    }
                    commands.entity(entity).despawn();

                    let nest = config.nest_position;
                    let caste = caste_ratios.pick_caste(rng.gen::<f32>());
                    let (speed, health, state, color) = match caste {
                        Caste::Worker => (
                            config.ant_speed_worker,
                            Health::worker(),
                            crate::components::ant::AntState::Foraging,
                            Color::srgb(0.1, 0.1, 0.1),
                        ),
                        Caste::Soldier => (
                            config.ant_speed_soldier,
                            Health::soldier(),
                            crate::components::ant::AntState::Defending,
                            Color::srgb(0.3, 0.1, 0.1),
                        ),
                        _ => (
                            config.ant_speed_worker,
                            Health::worker(),
                            crate::components::ant::AntState::Foraging,
                            Color::srgb(0.1, 0.1, 0.1),
                        ),
                    };

                    let offset_x = rng.gen_range(-15.0..15.0);
                    let offset_y = rng.gen_range(-15.0..15.0);

                    commands.spawn((
                        Sprite {
                            color,
                            custom_size: Some(Vec2::splat(4.0)),
                            ..default()
                        },
                        Transform::from_xyz(nest.x + offset_x, nest.y + offset_y, 2.0),
                        // New ants hatch onto the surface; hidden until sync_map_visibility.
                        Visibility::Hidden,
                        Ant { caste, state, age: 0.0, hunger: 0.0 },
                        Movement::with_random_direction(speed, &mut rng),
                        health,
                        ColonyMember { colony_id: 0 },
                        PositionHistory::default(),
                        TrailSense::default(),
                        MapId(registry.surface),
                    ));
                }
            }
        }
    }
}

fn update_colony_stats(
    mut stats: ResMut<ColonyStats>,
    ant_query: Query<&Ant>,
    brood_query: Query<&Brood>,
) {
    stats.workers = 0;
    stats.soldiers = 0;
    stats.drones = 0;
    stats.eggs = 0;
    stats.larvae = 0;
    stats.pupae = 0;

    for ant in &ant_query {
        match ant.caste {
            Caste::Worker => stats.workers += 1,
            Caste::Soldier => stats.soldiers += 1,
            Caste::Drone => stats.drones += 1,
            _ => {}
        }
    }

    for brood in &brood_query {
        match brood.stage {
            BroodStage::Egg => stats.eggs += 1,
            BroodStage::Larva => stats.larvae += 1,
            BroodStage::Pupa => stats.pupae += 1,
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

fn find_chamber_cells(grid: &NestGrid, kind: ChamberKind) -> Vec<(usize, usize)> {
    let mut cells = Vec::new();
    for y in 0..grid.height {
        for x in 0..grid.width {
            if grid.get(x, y) == CellType::Chamber(kind) {
                cells.push((x, y));
            }
        }
    }
    cells
}
