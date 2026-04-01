use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};

use crate::components::ant::Ant;
use crate::resources::simulation::SimClock;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Startup, setup_hud)
            .add_systems(Update, update_hud);
    }
}

#[derive(Component)]
struct HudText;

fn setup_hud(mut commands: Commands) {
    commands.spawn((
        Text::new("Colony"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        HudText,
    ));
}

fn update_hud(
    clock: Res<SimClock>,
    diagnostics: Res<DiagnosticsStore>,
    ant_query: Query<&Ant>,
    mut text_query: Query<&mut Text, With<HudText>>,
) {
    let Ok(mut text) = text_query.get_single_mut() else {
        return;
    };

    let ant_count = ant_query.iter().count();

    let fps = diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    **text = format!(
        "Ants: {}  |  Speed: {}  |  FPS: {:.0}  |  Time: {:.1}s\n\
         [Space] Pause/Play  [.] Cycle Speed  [WASD] Pan  [Scroll] Zoom",
        ant_count,
        clock.speed.label(),
        fps,
        clock.elapsed,
    );
}
