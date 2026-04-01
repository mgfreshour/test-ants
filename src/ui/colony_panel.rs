use bevy::prelude::*;

use crate::components::map::{MapKind, MapMarker};
use crate::resources::active_map::{ActiveMap, MapRegistry};
use crate::resources::colony::BehaviorSliders;

pub struct ColonyPanelPlugin;

impl Plugin for ColonyPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_panel)
            .add_systems(Update, (handle_slider_keys, update_panel_text));
    }
}

#[derive(Component)]
struct PanelText;

fn setup_panel(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        PanelText,
    ));
}

/// Adjust sliders on the player nest's BehaviorSliders component.
fn handle_slider_keys(
    input: Res<ButtonInput<KeyCode>>,
    registry: Res<MapRegistry>,
    mut sliders_query: Query<&mut BehaviorSliders, With<MapMarker>>,
) {
    let Ok(mut sliders) = sliders_query.get_mut(registry.player_nest) else { return };

    let step = 0.05;

    // 1/2 adjust forage vs nurse
    if input.just_pressed(KeyCode::Digit1) {
        sliders.forage = (sliders.forage + step).min(0.9);
        sliders.nurse = (sliders.nurse - step).max(0.0);
    }
    if input.just_pressed(KeyCode::Digit2) {
        sliders.nurse = (sliders.nurse + step).min(0.9);
        sliders.forage = (sliders.forage - step).max(0.0);
    }
    // 3/4 adjust dig vs defend
    if input.just_pressed(KeyCode::Digit3) {
        sliders.dig = (sliders.dig + step).min(0.9);
        sliders.defend = (sliders.defend - step).max(0.0);
    }
    if input.just_pressed(KeyCode::Digit4) {
        sliders.defend = (sliders.defend + step).min(0.9);
        sliders.dig = (sliders.dig - step).max(0.0);
    }
}

fn update_panel_text(
    registry: Res<MapRegistry>,
    active: Res<ActiveMap>,
    sliders_query: Query<&BehaviorSliders, With<MapMarker>>,
    mut query: Query<&mut Text, With<PanelText>>,
) {
    let Ok(mut text) = query.get_single_mut() else {
        return;
    };

    let view_label = match active.kind {
        MapKind::Surface => "Surface",
        MapKind::Nest { .. } => "Underground",
        MapKind::SpecialZone { .. } => "Special Zone",
    };

    // Show sliders for the player's nest.
    let Ok(sliders) = sliders_query.get(registry.player_nest) else { return };

    **text = format!(
        "View: {} [Tab]  |  Forage:{:.0}% [1+] Nurse:{:.0}% [2+] Dig:{:.0}% [3+] Defend:{:.0}% [4+]",
        view_label,
        sliders.forage * 100.0,
        sliders.nurse * 100.0,
        sliders.dig * 100.0,
        sliders.defend * 100.0,
    );
}
