use bevy::prelude::*;

/// Marks which map an entity lives on. Every ant, tile, brood, and other
/// "resident" entity carries this. Map entities themselves use `MapMarker`.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct MapId(pub Entity);

/// Describes the kind of map a map entity represents.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MapKind {
    Surface,
    Nest { colony_id: u32 },
    SpecialZone { zone_id: u32 },
}

/// Tag placed only on map entities (not on residents). Used to query map data
/// without conflicting with `MapId` queries on resident entities.
#[derive(Component)]
pub struct MapMarker;

/// A single side of a portal connection between two maps.
///
/// Portals come in pairs — each side is its own entity. When an ant is within
/// `PORTAL_RANGE` of `position` on its current map and the colony filter passes,
/// it transitions to `target_map` at `target_position`.
///
/// `colony_id = None` means any colony can use this portal (e.g. a neutral
/// passage). `colony_id = Some(id)` restricts access to that colony's ants.
#[derive(Component)]
pub struct MapPortal {
    /// The map this portal side belongs to.
    pub map: Entity,
    /// World-space position of the portal mouth on `map`.
    pub position: Vec2,
    /// Destination map.
    pub target_map: Entity,
    /// World-space position where the ant appears on `target_map`.
    pub target_position: Vec2,
    /// Colony restriction. `None` = open to all.
    pub colony_id: Option<u32>,
    /// The portal entity on the other side of this connection.
    pub pair: Entity,
}

/// How close (in world pixels) an ant must be to a portal mouth to trigger transition.
pub const PORTAL_RANGE: f32 = 20.0;

/// Spawn a matched portal pair linking `(map_a, pos_a)` ↔ `(map_b, pos_b)`.
/// Returns `(portal_a, portal_b)` — portal_a is on map_a, portal_b is on map_b.
pub fn spawn_portal_pair(
    commands: &mut Commands,
    map_a: Entity,
    pos_a: Vec2,
    map_b: Entity,
    pos_b: Vec2,
    colony_id: Option<u32>,
) -> (Entity, Entity) {
    // Pre-allocate both entities so each can reference the other's id.
    let portal_a = commands.spawn_empty().id();
    let portal_b = commands.spawn_empty().id();

    commands.entity(portal_a).insert(MapPortal {
        map: map_a,
        position: pos_a,
        target_map: map_b,
        target_position: pos_b,
        colony_id,
        pair: portal_b,
    });
    commands.entity(portal_b).insert(MapPortal {
        map: map_b,
        position: pos_b,
        target_map: map_a,
        target_position: pos_a,
        colony_id,
        pair: portal_a,
    });

    (portal_a, portal_b)
}
