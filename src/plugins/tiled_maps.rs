use std::collections::HashMap;

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::components::map::{MapId, MapKind, MapMarker, spawn_portal_pair};
use crate::components::nest::Queen;
use crate::components::terrain::FoodSource;
use crate::resources::active_map::MapRegistry;
use crate::resources::nest::{NestGrid, NEST_CELL_SIZE, NEST_HEIGHT, NEST_WIDTH};
use crate::resources::nest_pathfinding::NestPathCache;
use crate::resources::nest_pheromone::NestPheromoneGrid;
use crate::resources::simulation::SimConfig;
use crate::resources::surface_grid::SurfaceGrid;

pub struct TiledMapsPlugin;

impl Plugin for TiledMapsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TilemapPlugin)
            .init_resource::<SurfaceGrid>()
            .add_systems(Startup, load_maps);
    }
}

struct PortalData {
    portal_id: String,
    colony_id: Option<u32>,
    map: Entity,
    position: Vec2,
}

fn load_maps(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    registry: Res<MapRegistry>,
    config: Res<SimConfig>,
    mut nest_query: Query<
        (&mut NestGrid, &mut NestPheromoneGrid, &mut NestPathCache),
        With<MapMarker>,
    >,
    map_kind_query: Query<&MapKind, With<MapMarker>>,
    mut surface_grid: ResMut<SurfaceGrid>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut loader = tiled::Loader::new();
    let mut all_portals: Vec<PortalData> = Vec::new();

    // --- Surface ---
    if let Ok(map) = loader.load_tmx_map("assets/maps/colony/tiled/0001_Surface.tmx") {
        let map_height_px = (map.height * map.tile_height) as f32;

        load_surface_tiles(&map, &config, &mut surface_grid);
        spawn_tilemap(
            &mut commands, &asset_server, &map,
            "Terrain", "Terrain_tileset",
            Transform::from_xyz(0.0, 0.0, -10.0),
            Some(MapId(registry.surface)),
        );
        load_surface_entities(
            &mut commands,
            &map,
            map_height_px,
            registry.surface,
            &mut all_portals,
        );
    } else {
        warn!("Failed to load surface TMX");
    }

    let nest_offset = Vec3::new(
        -(NEST_WIDTH as f32 * NEST_CELL_SIZE) / 2.0,
        -(NEST_HEIGHT as f32 * NEST_CELL_SIZE) / 2.0,
        -10.0,
    );

    // --- Player Nest ---
    if let Ok(map) = loader.load_tmx_map("assets/maps/colony/tiled/0002_PlayerNest.tmx") {
        let map_height_px = (map.height * map.tile_height) as f32;

        load_nest_tiles(&map, registry.player_nest, &mut nest_query);
        spawn_tilemap(
            &mut commands, &asset_server, &map,
            "NestCells", "Nest_tileset",
            Transform::from_translation(nest_offset),
            Some(MapId(registry.player_nest)),
        );
        load_nest_entities(
            &mut commands,
            &map,
            map_height_px,
            registry.player_nest,
            &map_kind_query,
            &mut all_portals,
            &mut meshes,
            &mut materials,
        );
    } else {
        warn!("Failed to load player nest TMX");
    }

    // --- Red Nest ---
    if let Some(red_nest) = registry.red_nest {
        if let Ok(map) = loader.load_tmx_map("assets/maps/colony/tiled/0003_RedNest.tmx") {
            let map_height_px = (map.height * map.tile_height) as f32;

            load_nest_tiles(&map, red_nest, &mut nest_query);
            spawn_tilemap(
                &mut commands, &asset_server, &map,
                "NestCells", "Nest_tileset",
                Transform::from_translation(nest_offset),
                Some(MapId(red_nest)),
            );
            load_nest_entities(
                &mut commands,
                &map,
                map_height_px,
                red_nest,
                &map_kind_query,
                &mut all_portals,
                &mut meshes,
                &mut materials,
            );
        } else {
            warn!("Failed to load red nest TMX");
        }
    }

    // --- Wire portals ---
    wire_portals(&mut commands, &all_portals);
}

fn load_surface_tiles(
    map: &tiled::Map,
    config: &SimConfig,
    surface_grid: &mut SurfaceGrid,
) {
    let grid_w = (config.world_width / config.tile_size).ceil() as usize;
    let grid_h = (config.world_height / config.tile_size).ceil() as usize;
    let mut grid = SurfaceGrid::new(grid_w, grid_h, config.tile_size);

    if let Some(layer) = find_tile_layer(map, "Obstacles") {
        let width = map.width as usize;
        let height = map.height as usize;
        for tiled_y in 0..height {
            for x in 0..width {
                if layer.get_tile(x as i32, tiled_y as i32).is_some() {
                    let grid_y = height - 1 - tiled_y;
                    grid.set(x, grid_y, crate::resources::surface_grid::SurfaceCell::Blocked);
                }
            }
        }
    }

    *surface_grid = grid;
}

fn load_surface_entities(
    commands: &mut Commands,
    map: &tiled::Map,
    map_height_px: f32,
    surface_map: Entity,
    portals: &mut Vec<PortalData>,
) {
    for group in map.layers().filter_map(|l| l.as_object_layer()) {
        for obj in group.objects() {
            let obj_type = obj.user_type.as_str();
            let (obj_w, obj_h) = obj_size(&obj.shape);
            let bevy_x = obj.x + obj_w / 2.0;
            let bevy_y = map_height_px - (obj.y + obj_h / 2.0);

            match obj_type {
                "FoodSource" => {
                    let amount = get_float_prop(&obj.properties, "amount").unwrap_or(10.0);
                    let max_amount = get_float_prop(&obj.properties, "max_amount").unwrap_or(amount);
                    let size = get_float_prop(&obj.properties, "size").unwrap_or(12.0);

                    let color = if amount > 12.0 {
                        Color::srgb(0.85, 0.6, 0.15)
                    } else if amount > 8.0 {
                        Color::srgb(0.55, 0.35, 0.2)
                    } else {
                        Color::srgb(0.92, 0.87, 0.72)
                    };

                    commands.spawn((
                        Sprite {
                            color,
                            custom_size: Some(Vec2::splat(size)),
                            ..default()
                        },
                        Transform::from_xyz(bevy_x, bevy_y, 1.5),
                        FoodSource {
                            remaining: amount,
                            max: max_amount,
                        },
                        MapId(surface_map),
                    ));
                }
                "NestEntrance" => {
                    commands.spawn((
                        Sprite {
                            color: Color::srgb(0.35, 0.25, 0.15),
                            custom_size: Some(Vec2::splat(28.0)),
                            ..default()
                        },
                        Transform::from_xyz(bevy_x, bevy_y, 1.0),
                        crate::plugins::ant_sprites::NestMound,
                        MapId(surface_map),
                    ));
                }
                "PortalPoint" => {
                    let portal_id = get_string_prop(&obj.properties, "portal_id")
                        .unwrap_or_default();
                    let colony_id_raw = get_int_prop(&obj.properties, "colony_id").unwrap_or(-1);
                    let colony_id = if colony_id_raw >= 0 { Some(colony_id_raw as u32) } else { None };

                    portals.push(PortalData {
                        portal_id,
                        colony_id,
                        map: surface_map,
                        position: Vec2::new(bevy_x, bevy_y),
                    });
                }
                _ => {}
            }
        }
    }
}

fn load_nest_tiles(
    map: &tiled::Map,
    nest_map_entity: Entity,
    nest_query: &mut Query<
        (&mut NestGrid, &mut NestPheromoneGrid, &mut NestPathCache),
        With<MapMarker>,
    >,
) {
    let Some(layer) = find_tile_layer(map, "NestCells_values") else { return };
    let width = map.width as usize;
    let height = map.height as usize;

    let mut tiles = Vec::new();
    for y in 0..height {
        for x in 0..width {
            if let Some(tile) = layer.get_tile(x as i32, y as i32) {
                let value = tile.id() as i32 + 1;
                if value > 0 {
                    // from_intgrid expects LDtk coords (y=0 at bottom), so flip Tiled's top-down Y
                    let ldtk_y = (height - 1 - y) as i32;
                    tiles.push((x as i32, ldtk_y, value));
                }
            }
        }
    }

    let Ok((mut grid, mut phero_grid, mut path_cache)) = nest_query.get_mut(nest_map_entity) else {
        warn!("Nest entity not found for map loading");
        return;
    };

    *grid = NestGrid::from_intgrid(grid.width, grid.height, &tiles);

    let mut new_phero = NestPheromoneGrid::default();
    new_phero.seed_from_grid(&grid);
    *phero_grid = new_phero;

    path_cache.invalidate();
}

fn load_nest_entities(
    commands: &mut Commands,
    map: &tiled::Map,
    _map_height_px: f32,
    nest_map_entity: Entity,
    map_kind_query: &Query<&MapKind, With<MapMarker>>,
    portals: &mut Vec<PortalData>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    let colony_id = map_kind_query.get(nest_map_entity).ok()
        .and_then(|k| if let MapKind::Nest { colony_id } = k { Some(*colony_id) } else { None })
        .unwrap_or(0);

    let nest_offset = Vec2::new(
        -(NEST_WIDTH as f32 * NEST_CELL_SIZE) / 2.0,
        (NEST_HEIGHT as f32 * NEST_CELL_SIZE) / 2.0,
    );

    for group in map.layers().filter_map(|l| l.as_object_layer()) {
        for obj in group.objects() {
            let obj_type = obj.user_type.as_str();
            let (obj_w, obj_h) = obj_size(&obj.shape);
            let nest_x = nest_offset.x + obj.x + obj_w / 2.0;
            let nest_y = nest_offset.y - (obj.y + obj_h / 2.0);

            match obj_type {
                "QueenSpawn" => {
                    let color = if colony_id == 0 {
                        Color::srgb(0.8, 0.6, 0.1)
                    } else {
                        Color::srgb(0.8, 0.2, 0.1)
                    };

                    commands.spawn((
                        Mesh2d(meshes.add(Circle::new(6.0))),
                        MeshMaterial2d(materials.add(ColorMaterial::from(color))),
                        Transform::from_xyz(nest_x, nest_y, 3.0),
                        Visibility::Hidden,
                        Queen,
                        crate::components::nest::QueenHunger::default(),
                        crate::components::nest::QueenTask::Idle { timer: 0.0 },
                        crate::components::ant::ColonyMember { colony_id },
                        MapId(nest_map_entity),
                        crate::components::ant::Health {
                            current: 100.0,
                            max: 100.0,
                            last_damage_source: None,
                            last_attacker: None,
                        },
                    ));
                }
                "PortalPoint" => {
                    let portal_id = get_string_prop(&obj.properties, "portal_id")
                        .unwrap_or_default();
                    let colony_id_raw = get_int_prop(&obj.properties, "colony_id").unwrap_or(-1);
                    let cid = if colony_id_raw >= 0 { Some(colony_id_raw as u32) } else { None };

                    // Nest portal position uses nest coordinate system
                    portals.push(PortalData {
                        portal_id,
                        colony_id: cid,
                        map: nest_map_entity,
                        position: Vec2::new(nest_x, nest_y),
                    });
                }
                _ => {}
            }
        }
    }
}

fn wire_portals(commands: &mut Commands, portals: &[PortalData]) {
    let mut by_id: HashMap<&str, Vec<&PortalData>> = HashMap::new();
    for p in portals {
        by_id.entry(&p.portal_id).or_default().push(p);
    }

    for (_portal_id, group) in &by_id {
        if group.len() != 2 {
            continue;
        }
        let a = group[0];
        let b = group[1];
        spawn_portal_pair(
            commands,
            a.map,
            a.position,
            b.map,
            b.position,
            a.colony_id.or(b.colony_id),
        );
    }
}

// --- Tilemap rendering ---

fn spawn_tilemap(
    commands: &mut Commands,
    asset_server: &AssetServer,
    map: &tiled::Map,
    layer_name: &str,
    tileset_name: &str,
    transform: Transform,
    map_id: Option<MapId>,
) {
    let Some(layer) = find_tile_layer(map, layer_name) else {
        warn!("Layer '{}' not found", layer_name);
        return;
    };

    let tileset = map.tilesets().iter().find(|ts| ts.name == tileset_name);
    let Some(tileset) = tileset else {
        warn!("Tileset '{}' not found", tileset_name);
        return;
    };

    let image_source = tileset.image.as_ref()
        .map(|img| img.source.to_string_lossy().to_string())
        .unwrap_or_default();

    // The tiled crate resolves image paths relative to CWD. Normalize and
    // strip the "assets/" prefix so Bevy's asset server can find them.
    let image_path = normalize_to_asset_path(&image_source);
    let texture_handle: Handle<Image> = asset_server.load(&image_path);

    let map_width = map.width;
    let map_height = map.height;
    let tile_w = map.tile_width as f32;
    let tile_h = map.tile_height as f32;

    let map_size = TilemapSize { x: map_width, y: map_height };
    let mut tile_storage = TileStorage::empty(map_size);
    let tilemap_entity = commands.spawn_empty().id();

    for y in 0..map_height {
        for x in 0..map_width {
            if let Some(tile) = layer.get_tile(x as i32, y as i32) {
                if tile.get_tileset().name != tileset_name {
                    continue;
                }
                let local_id = tile.id();
                let tile_pos = TilePos { x, y: map_height - 1 - y };
                let tile_entity = commands.spawn((
                    TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(tilemap_entity),
                        texture_index: TileTextureIndex(local_id),
                        ..default()
                    },
                )).id();
                if let Some(mid) = map_id {
                    commands.entity(tile_entity).insert(mid);
                }
                tile_storage.set(&tile_pos, tile_entity);
            }
        }
    }

    let tilemap_texture = TilemapTexture::Single(texture_handle);
    let grid_size = TilemapGridSize { x: tile_w, y: tile_h };
    let tile_size = TilemapTileSize { x: tile_w, y: tile_h };
    let map_type = TilemapType::Square;

    let mut tilemap_cmd = commands.entity(tilemap_entity);
    tilemap_cmd.insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: tilemap_texture,
        tile_size,
        transform,
        anchor: TilemapAnchor::BottomLeft,
        ..default()
    });
    if let Some(mid) = map_id {
        tilemap_cmd.insert(mid);
    }
}

// --- Helpers ---

fn find_tile_layer<'a>(map: &'a tiled::Map, name: &str) -> Option<tiled::TileLayer<'a>> {
    map.layers()
        .find(|l| l.name == name)
        .and_then(|l| l.as_tile_layer())
}

fn normalize_to_asset_path(raw: &str) -> String {
    use std::path::PathBuf;
    let path = PathBuf::from(raw);
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => { parts.pop(); }
            std::path::Component::Normal(s) => parts.push(s.to_string_lossy().to_string()),
            _ => {}
        }
    }
    let joined = parts.join("/");
    joined.strip_prefix("assets/").unwrap_or(&joined).to_string()
}

fn obj_size(shape: &tiled::ObjectShape) -> (f32, f32) {
    match shape {
        tiled::ObjectShape::Rect { width, height } => (*width, *height),
        _ => (0.0, 0.0),
    }
}

fn get_float_prop(props: &tiled::Properties, key: &str) -> Option<f32> {
    match props.get(key) {
        Some(tiled::PropertyValue::FloatValue(v)) => Some(*v),
        Some(tiled::PropertyValue::StringValue(s)) => s.parse().ok(),
        Some(tiled::PropertyValue::IntValue(v)) => Some(*v as f32),
        _ => None,
    }
}

fn get_int_prop(props: &tiled::Properties, key: &str) -> Option<i32> {
    match props.get(key) {
        Some(tiled::PropertyValue::IntValue(v)) => Some(*v),
        Some(tiled::PropertyValue::StringValue(s)) => s.parse().ok(),
        Some(tiled::PropertyValue::FloatValue(v)) => Some(*v as i32),
        _ => None,
    }
}

fn get_string_prop(props: &tiled::Properties, key: &str) -> Option<String> {
    match props.get(key) {
        Some(tiled::PropertyValue::StringValue(s)) => Some(s.clone()),
        Some(tiled::PropertyValue::IntValue(v)) => Some(v.to_string()),
        _ => None,
    }
}
