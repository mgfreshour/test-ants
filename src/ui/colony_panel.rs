use bevy::prelude::*;

use crate::resources::colony::BehaviorSliders;
use crate::plugins::nest::GameView;

pub struct ColonyPanelPlugin;

impl Plugin for ColonyPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BehaviorSliders>()
            .add_systems(Startup, setup_panel)
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

fn handle_slider_keys(
    input: Res<ButtonInput<KeyCode>>,
    mut sliders: ResMut<BehaviorSliders>,
) {
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
    sliders: Res<BehaviorSliders>,
    view: Res<State<GameView>>,
    mut query: Query<&mut Text, With<PanelText>>,
) {
    let Ok(mut text) = query.get_single_mut() else {
        return;
    };

    let view_label = match view.get() {
        GameView::Surface => "Surface",
        GameView::Underground => "Underground",
    };

    **text = format!(
        "View: {} [Tab]  |  Forage:{:.0}% [1+] Nurse:{:.0}% [2+] Dig:{:.0}% [3+] Defend:{:.0}% [4+]",
        view_label,
        sliders.forage * 100.0,
        sliders.nurse * 100.0,
        sliders.dig * 100.0,
        sliders.defend * 100.0,
    );
}
