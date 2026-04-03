use std::collections::HashMap;

use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use crate::components::map::{MapId, MapKind, MapMarker, spawn_portal_pair};
use crate::components::nest::Queen;
use crate::components::terrain::FoodSource;
use crate::resources::active_map::MapRegistry;
use crate::resources::nest::{NestGrid, NEST_CELL_SIZE, NEST_HEIGHT, NEST_WIDTH};
use crate::resources::nest_pathfinding::NestPathCache;
use crate::resources::nest_pheromone::NestPheromoneGrid;

pub struct LdtkMapsPlugin;

/// Marker: this nest map's NestGrid has been rebuilt from LDtk IntGrid data.
#[derive(Component)]
struct NestLdtkSynced;

/// Marker: this LDtk entity has been processed into game components.
#[derive(Component)]
struct LdtkEntityProcessed;

/// Intermediate component for portal wiring.
#[derive(Component)]
struct LdtkPortalPoint {
    portal_id: String,
    colony_id: Option<u32>,
    map: Entity,
    position: Vec2,
}

/// Resource: portals have been wired (prevents re-wiring every frame).
#[derive(Resource)]
struct LdtkPortalsWired;

/// Resource: queens have been spawned from LDtk markers.
#[derive(Resource)]
struct LdtkQueensSpawned;

impl Plugin for LdtkMapsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LdtkPlugin)
            .insert_resource(LdtkSettings {
                level_spawn_behavior: LevelSpawnBehavior::UseZeroTranslation,
                set_clear_color: SetClearColor::No,
                int_grid_rendering: IntGridRendering::Invisible,
                ..default()
            })
            .add_systems(Startup, spawn_ldtk_world)
            .add_systems(
                Update,
                (
                    sync_ldtk_nest_tiles,
                    process_ldtk_entities,
                    wire_ldtk_portals,
                    spawn_queens_from_ldtk,
                )
                    .chain(),
            );
    }
}

/// Spawns LdtkWorldBundles for the surface and each nest level.
/// Each gets a MapId so InheritedVisibility cascades show/hide correctly.
fn spawn_ldtk_world(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    registry: Res<MapRegistry>,
) {
    // Surface
    commands.spawn((
        LdtkWorldBundle {
            ldtk_handle: asset_server.load("maps/colony.ldtk").into(),
            level_set: LevelSet::from_iids(["9590239f-aaf8-463e-a9d3-2a54c2d23d3a"]),
            transform: Transform::default(),
            ..default()
        },
        MapId(registry.surface),
    ));

    // Nest offset: align bevy_ecs_tilemap grid with nest_grid_to_world coordinates.
    // GridCoords(0,39) [top-left in LDtk] should map to nest_grid_to_world(0,0) = (-472, 312).
    // Tilemap places GridCoords(0,39) at (8, 632) locally → offset = (-480, -320).
    let nest_offset = Vec3::new(
        -(NEST_WIDTH as f32 * NEST_CELL_SIZE) / 2.0,
        -(NEST_HEIGHT as f32 * NEST_CELL_SIZE) / 2.0,
        0.0,
    );

    // Player nest
    commands.spawn((
        LdtkWorldBundle {
            ldtk_handle: asset_server.load("maps/colony.ldtk").into(),
            level_set: LevelSet::from_iids(["1228f450-47bb-4e81-83fd-246ad316535b"]),
            transform: Transform::from_translation(nest_offset),
            ..default()
        },
        MapId(registry.player_nest),
    ));

    // Red nest
    if let Some(red_nest) = registry.red_nest {
        commands.spawn((
            LdtkWorldBundle {
                ldtk_handle: asset_server.load("maps/colony.ldtk").into(),
                level_set: LevelSet::from_iids(["29005165-cb0a-48eb-8899-991f1fa612a2"]),
                transform: Transform::from_translation(nest_offset),
                ..default()
            },
            MapId(red_nest),
        ));
    }
}

/// Tag newly-spawned IntGrid tile entities with MapId from their ancestor
/// LdtkWorldBundle, and rebuild NestGrid from the IntGrid data.
///
/// Hierarchy: Tile -ChildOf→ Layer -ChildOf→ Level -ChildOf→ LdtkWorldBundle (has MapId).
fn sync_ldtk_nest_tiles(
    mut commands: Commands,
    new_tiles: Query<(Entity, &IntGridCell, &GridCoords, &ChildOf), Without<MapId>>,
    parent_query: Query<&ChildOf>,
    map_id_query: Query<&MapId>,
    mut nest_query: Query<
        (Entity, &mut NestGrid, &mut NestPheromoneGrid, &mut NestPathCache),
        (With<MapMarker>, Without<NestLdtkSynced>),
    >,
) {
    // Phase 1: Walk each untagged tile up to its LdtkWorldBundle, collect data.
    let mut tiles_by_map: HashMap<Entity, Vec<(i32, i32, i32)>> = HashMap::new();

    for (tile_entity, cell, coords, tile_parent) in &new_tiles {
        // tile → layer (ChildOf)
        let layer_entity = tile_parent.parent();
        // layer → level (ChildOf)
        let Ok(layer_parent) = parent_query.get(layer_entity) else { continue };
        let level_entity = layer_parent.parent();
        // level → world (ChildOf)
        let Ok(level_parent) = parent_query.get(level_entity) else { continue };
        let world_entity = level_parent.parent();
        // Get MapId from world
        let Ok(&map_id) = map_id_query.get(world_entity) else { continue };

        commands.entity(tile_entity).insert(map_id);
        tiles_by_map
            .entry(map_id.0)
            .or_default()
            .push((coords.x, coords.y, cell.value));
    }

    // Phase 2: Rebuild NestGrid for nest maps that received tiles.
    for (map_entity, mut grid, mut phero_grid, mut path_cache) in &mut nest_query {
        let Some(tiles) = tiles_by_map.get(&map_entity) else {
            continue;
        };
        if tiles.is_empty() {
            continue;
        }

        *grid = NestGrid::from_intgrid(grid.width, grid.height, tiles);

        let mut new_phero = NestPheromoneGrid::default();
        new_phero.seed_from_grid(&grid);
        *phero_grid = new_phero;

        path_cache.invalidate();
        commands.entity(map_entity).insert(NestLdtkSynced);
    }
}

/// Walk ChildOf hierarchy from an entity up 3 levels to find the LdtkWorldBundle's MapId.
fn resolve_map_id(
    entity: Entity,
    parent_query: &Query<&ChildOf>,
    map_id_query: &Query<&MapId>,
) -> Option<MapId> {
    // entity → layer (ChildOf)
    let layer = parent_query.get(entity).ok()?.parent();
    // layer → level (ChildOf)
    let level = parent_query.get(layer).ok()?.parent();
    // level → world (ChildOf)
    let world = parent_query.get(level).ok()?.parent();
    map_id_query.get(world).ok().copied()
}

/// Compute the world-space position of a spawned LDtk entity.
/// Walks hierarchy to find the LdtkWorldBundle's transform offset.
fn resolve_world_position(
    entity_transform: &Transform,
    entity: Entity,
    parent_query: &Query<&ChildOf>,
    transform_query: &Query<&Transform>,
) -> Vec2 {
    // Walk up to find LdtkWorldBundle (3 levels up) and get its transform.
    let mut offset = Vec3::ZERO;
    let mut current = entity;
    for _ in 0..3 {
        if let Ok(child_of) = parent_query.get(current) {
            current = child_of.parent();
            if let Ok(parent_tf) = transform_query.get(current) {
                offset += parent_tf.translation;
            }
        }
    }
    let world_pos = entity_transform.translation + offset;
    world_pos.truncate()
}

/// Process newly-spawned LDtk entities: tag with MapId and create game components.
fn process_ldtk_entities(
    mut commands: Commands,
    registry: Res<MapRegistry>,
    new_entities: Query<(Entity, &EntityInstance, &Transform), Without<LdtkEntityProcessed>>,
    parent_query: Query<&ChildOf>,
    map_id_query: Query<&MapId>,
    transform_query: Query<&Transform>,
) {
    for (entity, instance, transform) in &new_entities {
        let Some(map_id) = resolve_map_id(entity, &parent_query, &map_id_query) else {
            continue;
        };
        let world_pos = resolve_world_position(transform, entity, &parent_query, &transform_query);

        commands.entity(entity).insert((map_id, LdtkEntityProcessed));

        match instance.identifier.as_str() {
            "FoodSource" => {
                let amount = instance.get_float_field("amount").copied().unwrap_or(10.0);
                let max_amount = instance.get_float_field("max_amount").copied().unwrap_or(amount);
                let size = instance.get_float_field("size").copied().unwrap_or(12.0);

                // Only spawn food on surface
                if map_id.0 == registry.surface {
                    let color = if amount > 12.0 {
                        Color::srgb(0.85, 0.6, 0.15) // fruit
                    } else if amount > 8.0 {
                        Color::srgb(0.55, 0.35, 0.2) // dead insect
                    } else {
                        Color::srgb(0.92, 0.87, 0.72) // crumbs
                    };

                    commands.entity(entity).insert((
                        Sprite {
                            color,
                            custom_size: Some(Vec2::splat(size)),
                            ..default()
                        },
                        Transform::from_xyz(world_pos.x, world_pos.y, 1.5),
                        FoodSource {
                            remaining: amount,
                            max: max_amount,
                        },
                    ));
                }
            }
            "PortalPoint" => {
                let portal_id = instance.get_string_field("portal_id")
                    .map(|s| s.clone())
                    .unwrap_or_default();
                let colony_id_raw = instance.get_int_field("colony_id").copied().unwrap_or(-1);
                let colony_id = if colony_id_raw >= 0 { Some(colony_id_raw as u32) } else { None };

                commands.entity(entity).insert(LdtkPortalPoint {
                    portal_id,
                    colony_id,
                    map: map_id.0,
                    position: world_pos,
                });
            }
            "NestEntrance" => {
                // Spawn visual mound marker on surface
                if map_id.0 == registry.surface {
                    let mound_color = Color::srgb(0.35, 0.25, 0.15);
                    let hole_color = Color::srgb(0.08, 0.05, 0.02);

                    commands.entity(entity).insert((
                        Sprite {
                            color: mound_color,
                            custom_size: Some(Vec2::splat(28.0)),
                            ..default()
                        },
                        Transform::from_xyz(world_pos.x, world_pos.y, 1.0),
                    ));

                    // Inner dark hole as child
                    commands.spawn((
                        Sprite {
                            color: hole_color,
                            custom_size: Some(Vec2::splat(14.0)),
                            ..default()
                        },
                        Transform::from_xyz(world_pos.x, world_pos.y, 1.1),
                        map_id,
                    ));
                }
            }
            "QueenSpawn" => {
                // Handled by spawn_queens_from_ldtk
            }
            _ => {}
        }
    }
}

/// Wire portal pairs from LdtkPortalPoint entities grouped by portal_id.
fn wire_ldtk_portals(
    mut commands: Commands,
    portal_points: Query<&LdtkPortalPoint>,
    existing: Option<Res<LdtkPortalsWired>>,
) {
    if existing.is_some() {
        return;
    }

    // Need at least 2 portal points to wire a pair
    let points: Vec<&LdtkPortalPoint> = portal_points.iter().collect();
    if points.len() < 2 {
        return;
    }

    // Group by portal_id
    let mut by_id: HashMap<&str, Vec<&LdtkPortalPoint>> = HashMap::new();
    for p in &points {
        by_id.entry(&p.portal_id).or_default().push(p);
    }

    for (_portal_id, group) in &by_id {
        if group.len() != 2 {
            continue;
        }
        let a = group[0];
        let b = group[1];
        spawn_portal_pair(
            &mut commands,
            a.map,
            a.position,
            b.map,
            b.position,
            a.colony_id.or(b.colony_id),
        );
    }

    commands.insert_resource(LdtkPortalsWired);
}

/// Spawn queens from LDtk QueenSpawn marker entities.
fn spawn_queens_from_ldtk(
    mut commands: Commands,
    existing: Option<Res<LdtkQueensSpawned>>,
    queen_markers: Query<(Entity, &EntityInstance, &Transform, &MapId), (With<LdtkEntityProcessed>, Without<Queen>)>,
    parent_query: Query<&ChildOf>,
    transform_query: Query<&Transform>,
    map_kind_query: Query<&MapKind, With<MapMarker>>,
    existing_queens: Query<&Queen>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if existing.is_some() {
        return;
    }

    let mut spawned_any = false;

    for (entity, instance, transform, map_id) in &queen_markers {
        if instance.identifier != "QueenSpawn" {
            continue;
        }

        let world_pos = resolve_world_position(transform, entity, &parent_query, &transform_query);

        // Derive colony_id from the nest's MapKind
        let colony_id = map_kind_query.get(map_id.0).ok()
            .and_then(|k| if let MapKind::Nest { colony_id } = k { Some(*colony_id) } else { None })
            .unwrap_or(0);

        let color = if colony_id == 0 {
            Color::srgb(0.8, 0.6, 0.1) // gold
        } else {
            Color::srgb(0.8, 0.2, 0.1) // red-gold
        };

        commands.spawn((
            Mesh2d(meshes.add(Circle::new(6.0))),
            MeshMaterial2d(materials.add(ColorMaterial::from(color))),
            Transform::from_xyz(world_pos.x, world_pos.y, 3.0),
            Visibility::Hidden,
            Queen,
            crate::components::nest::QueenHunger::default(),
            crate::components::ant::ColonyMember { colony_id },
            *map_id,
            crate::components::ant::Health { current: 100.0, max: 100.0 },
        ));

        spawned_any = true;
    }

    if spawned_any || !existing_queens.is_empty() {
        commands.insert_resource(LdtkQueensSpawned);
    }
}
