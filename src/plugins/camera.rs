use std::collections::HashSet;

use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use bevy_ecs_tilemap::prelude::TileStorage;

use crate::components::map::MapKind;
use crate::plugins::player::PlayerMode;
use crate::resources::active_map::ActiveMap;
use crate::resources::nest::{NEST_CELL_SIZE, NEST_HEIGHT, NEST_WIDTH};
use crate::resources::simulation::SimConfig;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ZClipWarnings>()
            .add_systems(Startup, setup_camera)
            .add_systems(
                Update,
                (camera_pan, camera_zoom, camera_clamp).chain(),
            )
            .add_systems(Update, warn_on_z_clip);
    }
}

#[derive(Component)]
pub struct MainCamera;

fn setup_camera(mut commands: Commands, config: Res<SimConfig>) {
    let center_x = config.world_width / 2.0;
    let center_y = config.world_height / 2.0;

    commands.spawn((
        Camera2d,
        Transform::from_xyz(center_x, center_y, 999.0),
        Projection::from(OrthographicProjection {
            scale: 1.0,
            near: -1000.0,
            far: 2000.0,
            ..OrthographicProjection::default_2d()
        }),
        MainCamera,
    ));
}

const PAN_SPEED: f32 = 500.0;
const ZOOM_SPEED: f32 = 0.02;
const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 0.5;

fn camera_pan(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mode: Res<PlayerMode>,
    mut query: Query<(&mut Transform, &Projection), With<MainCamera>>,
) {
    if mode.controlling {
        return;
    }
    let Ok((mut transform, projection)) = query.single_mut() else {
        return;
    };

    let scale = match projection {
        Projection::Orthographic(ref ortho) => ortho.scale,
        _ => 1.0,
    };

    let mut delta = Vec2::ZERO;

    if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft) {
        delta.x -= 1.0;
    }
    if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight) {
        delta.x += 1.0;
    }
    if input.pressed(KeyCode::KeyW) || input.pressed(KeyCode::ArrowUp) {
        delta.y += 1.0;
    }
    if input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown) {
        delta.y -= 1.0;
    }

    if delta != Vec2::ZERO {
        let speed = PAN_SPEED * scale * time.delta_secs();
        transform.translation.x += delta.x * speed;
        transform.translation.y += delta.y * speed;
    }
}

fn camera_zoom(
    mut scroll_events: MessageReader<MouseWheel>,
    mut query: Query<&mut Projection, With<MainCamera>>,
) {
    let Ok(mut projection) = query.single_mut() else {
        return;
    };

    if let Projection::Orthographic(ref mut ortho) = *projection {
        for event in scroll_events.read() {
            let zoom_delta = -event.y * ZOOM_SPEED;
            ortho.scale = (ortho.scale + zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);
        }
    }
}

/// Tracks which entities we've already warned about so we don't spam the log
/// every frame for persistent violations.
#[derive(Resource, Default)]
struct ZClipWarnings {
    warned: HashSet<Entity>,
}

/// Warns when any renderable entity's global-space z falls outside the main
/// camera's orthographic frustum. In a 2D game every visible object should sit
/// inside the camera's z range; anything outside will silently not render.
///
/// Checked entity kinds: sprites and tilemap storages (the two rendering
/// primitives we use). Each offending entity is logged once.
fn warn_on_z_clip(
    camera: Query<(&Transform, &Projection), With<MainCamera>>,
    sprites: Query<(Entity, &GlobalTransform, Option<&Name>), With<Sprite>>,
    tilemaps: Query<(Entity, &GlobalTransform, Option<&Name>), With<TileStorage>>,
    mut warnings: ResMut<ZClipWarnings>,
) {
    let Ok((cam_tf, projection)) = camera.single() else {
        return;
    };
    let Projection::Orthographic(ref ortho) = projection else {
        return;
    };

    // For a 2D orthographic camera looking down -Z, the visible world-space z
    // range is [camera.z - far, camera.z - near].
    let cam_z = cam_tf.translation.z;
    let z_min = cam_z - ortho.far;
    let z_max = cam_z - ortho.near;

    let mut check = |entity: Entity, gtf: &GlobalTransform, name: Option<&Name>, kind: &str| {
        let z = gtf.translation().z;
        if z < z_min || z > z_max {
            if warnings.warned.insert(entity) {
                let label = name
                    .map(|n| n.as_str().to_string())
                    .unwrap_or_else(|| format!("{entity:?}"));
                warn!(
                    "{} '{}' at z={:.3} is outside camera z range [{:.3}, {:.3}] \
                     (camera.z={:.3}, near={:.3}, far={:.3}). It will not render.",
                    kind, label, z, z_min, z_max, cam_z, ortho.near, ortho.far
                );
            }
        }
    };

    for (entity, gtf, name) in &sprites {
        check(entity, gtf, name, "Sprite");
    }
    for (entity, gtf, name) in &tilemaps {
        check(entity, gtf, name, "Tilemap");
    }
}

fn camera_clamp(
    config: Res<SimConfig>,
    active: Option<Res<ActiveMap>>,
    mut query: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(mut transform) = query.single_mut() else {
        return;
    };

    let is_nest = active
        .as_ref()
        .map_or(false, |a| matches!(a.kind, MapKind::Nest { .. }));

    if is_nest {
        let half_w = (NEST_WIDTH as f32 * NEST_CELL_SIZE) / 2.0;
        let half_h = (NEST_HEIGHT as f32 * NEST_CELL_SIZE) / 2.0;
        transform.translation.x = transform.translation.x.clamp(-half_w, half_w);
        transform.translation.y = transform.translation.y.clamp(-half_h, half_h);
    } else {
        transform.translation.x = transform.translation.x.clamp(0.0, config.world_width);
        transform.translation.y = transform.translation.y.clamp(0.0, config.world_height);
    }
}
