use std::collections::HashMap;

use bevy::prelude::*;

use crate::components::map::MapKind;

// ── Run conditions ────────────────────────────────────────────────────

/// `run_if` condition: player is currently viewing the surface map.
pub fn viewing_surface(active: Option<Res<ActiveMap>>) -> bool {
    active.map_or(true, |a| a.kind == MapKind::Surface)
}

/// `run_if` condition: player is currently viewing any nest map.
pub fn viewing_nest(active: Option<Res<ActiveMap>>) -> bool {
    active.map_or(false, |a| matches!(a.kind, MapKind::Nest { .. }))
}

/// The map currently being viewed by the player.
#[derive(Resource)]
pub struct ActiveMap {
    pub entity: Entity,
    pub kind: MapKind,
}

/// Index of all maps in the world. Used by Tab-cycling and by transition
/// systems that need to locate well-known maps by role.
#[derive(Resource)]
pub struct MapRegistry {
    /// The surface overworld map.
    pub surface: Entity,
    /// The player colony's underground nest.
    pub player_nest: Entity,
    /// All maps in cycle order (surface first, then nests in colony_id order).
    pub maps: Vec<Entity>,
}

/// Per-map saved camera state, keyed by map entity.
/// When the player switches maps the current camera position/zoom is stored
/// here so it can be restored on return.
#[derive(Resource, Default)]
pub struct SavedCameraStates(pub HashMap<Entity, SavedCamera>);

pub struct SavedCamera {
    pub position: Vec2,
    pub scale: f32,
}
