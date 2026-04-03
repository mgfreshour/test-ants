use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::*;

use crate::components::map::MapId;
use crate::resources::active_map::MapRegistry;

pub struct LdtkMapsPlugin;

impl Plugin for LdtkMapsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LdtkPlugin)
            .insert_resource(LdtkSettings {
                level_spawn_behavior: LevelSpawnBehavior::UseZeroTranslation,
                set_clear_color: SetClearColor::No,
                int_grid_rendering: IntGridRendering::Colorful,
                ..default()
            })
            .add_systems(Startup, spawn_ldtk_world);
    }
}

/// Spawns the LDtk world bundle tagged with MapId(surface) so that
/// Bevy's InheritedVisibility cascades the show/hide to all child
/// tilemap entities when the player switches maps.
fn spawn_ldtk_world(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    registry: Res<MapRegistry>,
) {
    commands.spawn((
        LdtkWorldBundle {
            ldtk_handle: asset_server.load("maps/colony.ldtk").into(),
            level_set: LevelSet::from_iids(["9590239f-aaf8-463e-a9d3-2a54c2d23d3a"]),
            transform: Transform::default(),
            ..default()
        },
        MapId(registry.surface),
    ));
}
