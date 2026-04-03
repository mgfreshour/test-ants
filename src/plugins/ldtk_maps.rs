use std::collections::HashMap;

use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use crate::components::map::{MapId, MapMarker};
use crate::resources::active_map::MapRegistry;
use crate::resources::nest::{NestGrid, NEST_CELL_SIZE, NEST_HEIGHT, NEST_WIDTH};
use crate::resources::nest_pathfinding::NestPathCache;
use crate::resources::nest_pheromone::NestPheromoneGrid;

pub struct LdtkMapsPlugin;

/// Marker: this nest map's NestGrid has been rebuilt from LDtk IntGrid data.
#[derive(Component)]
struct NestLdtkSynced;

impl Plugin for LdtkMapsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LdtkPlugin)
            .insert_resource(LdtkSettings {
                level_spawn_behavior: LevelSpawnBehavior::UseZeroTranslation,
                set_clear_color: SetClearColor::No,
                int_grid_rendering: IntGridRendering::Colorful,
                ..default()
            })
            .add_systems(Startup, spawn_ldtk_world)
            .add_systems(Update, sync_ldtk_nest_tiles);
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
