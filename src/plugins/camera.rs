use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;

use crate::plugins::player::PlayerMode;
use crate::resources::active_map::viewing_surface;
use crate::resources::simulation::SimConfig;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
            .add_systems(
                Update,
                (camera_pan, camera_zoom, camera_clamp)
                    .chain()
                    .run_if(viewing_surface),
            );
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
        OrthographicProjection {
            scale: 1.0,
            ..OrthographicProjection::default_2d()
        },
        MainCamera,
    ));
}

const PAN_SPEED: f32 = 500.0;
const ZOOM_SPEED: f32 = 0.1;
const MIN_ZOOM: f32 = 0.2;
const MAX_ZOOM: f32 = 5.0;

fn camera_pan(
    input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mode: Res<PlayerMode>,
    mut query: Query<(&mut Transform, &OrthographicProjection), With<MainCamera>>,
) {
    if mode.controlling {
        return;
    }
    let Ok((mut transform, projection)) = query.get_single_mut() else {
        return;
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
        let speed = PAN_SPEED * projection.scale * time.delta_secs();
        transform.translation.x += delta.x * speed;
        transform.translation.y += delta.y * speed;
    }
}

fn camera_zoom(
    mut scroll_events: EventReader<MouseWheel>,
    mut query: Query<&mut OrthographicProjection, With<MainCamera>>,
) {
    let Ok(mut projection) = query.get_single_mut() else {
        return;
    };

    for event in scroll_events.read() {
        let zoom_delta = -event.y * ZOOM_SPEED;
        projection.scale = (projection.scale + zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);
    }
}

fn camera_clamp(
    config: Res<SimConfig>,
    mut query: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(mut transform) = query.get_single_mut() else {
        return;
    };

    transform.translation.x = transform
        .translation
        .x
        .clamp(0.0, config.world_width);
    transform.translation.y = transform
        .translation
        .y
        .clamp(0.0, config.world_height);
}
