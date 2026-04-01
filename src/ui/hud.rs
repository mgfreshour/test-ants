use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};

use crate::components::ant::{Ant, AntState, CarriedItem};
use crate::plugins::ant_ai::ColonyFood;
use crate::plugins::pheromone::{OverlayDisplay, OverlayState};
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
    overlay: Res<OverlayState>,
    colony_food: Res<ColonyFood>,
    diagnostics: Res<DiagnosticsStore>,
    ant_query: Query<(&Ant, Option<&CarriedItem>)>,
    mut text_query: Query<&mut Text, With<HudText>>,
) {
    let Ok(mut text) = text_query.get_single_mut() else {
        return;
    };

    let mut total = 0;
    let mut foraging = 0;
    let mut returning = 0;
    for (ant, _carried) in &ant_query {
        total += 1;
        match ant.state {
            AntState::Foraging => foraging += 1,
            AntState::Returning => returning += 1,
            _ => {}
        }
    }

    let fps = diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let overlay_label = if overlay.visible {
        match overlay.display_type {
            OverlayDisplay::All => "All",
            OverlayDisplay::Home => "Home",
            OverlayDisplay::Food => "Food",
            OverlayDisplay::Alarm => "Alarm",
            OverlayDisplay::Trail => "Trail",
        }
    } else {
        "Off"
    };

    **text = format!(
        "Ants: {} (forage:{} return:{})  |  Food: {:.0}  |  Speed: {}  |  Overlay: {}  |  FPS: {:.0}\n\
         [Space] Pause  [.] Speed  [H] Overlay  [WASD] Pan  [Scroll] Zoom",
        total,
        foraging,
        returning,
        colony_food.stored,
        clock.speed.label(),
        overlay_label,
        fps,
    );
}
