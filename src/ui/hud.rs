use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};

use crate::components::ant::{Ant, AntState, CarriedItem, PlayerControlled};
use crate::plugins::ant_ai::ColonyFood;
use crate::plugins::nest::GameView;
use crate::plugins::player::{FollowerCount, PlayerMode};
use crate::plugins::pheromone::{OverlayDisplay, OverlayState};
use crate::resources::colony::ColonyStats;
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
    stats: Res<ColonyStats>,
    player_mode: Res<PlayerMode>,
    _followers: Res<FollowerCount>,
    view: Res<State<GameView>>,
    diagnostics: Res<DiagnosticsStore>,
    ant_query: Query<(&Ant, Option<&CarriedItem>)>,
    player_query: Query<Option<&CarriedItem>, With<PlayerControlled>>,
    mut text_query: Query<&mut Text, With<HudText>>,
) {
    let Ok(mut text) = text_query.get_single_mut() else {
        return;
    };

    let fps = diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let pop = stats.workers + stats.soldiers + stats.drones;
    let brood = stats.eggs + stats.larvae + stats.pupae;

    match view.get() {
        GameView::Underground => {
            **text = format!(
                "[NEST] Pop:{} Brood:{}  |  Food:{:.0}  |  {}  |  FPS:{:.0}\n\
                 Tab:surface",
                pop, brood,
                colony_food.stored,
                clock.speed.label(),
                fps,
            );
        }
        GameView::Surface => {
            let mut foraging = 0u32;
            let mut returning = 0u32;
            let mut following = 0u32;
            for (ant, _carried) in &ant_query {
                match ant.state {
                    AntState::Foraging => foraging += 1,
                    AntState::Returning => returning += 1,
                    AntState::Following => following += 1,
                    _ => {}
                }
            }

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

            let mode_str = if player_mode.controlling { "ANT" } else { "CAM" };

            let player_carrying = player_query
                .get_single()
                .ok()
                .flatten()
                .map(|c| format!(" Carry:{:.0}", c.food_amount))
                .unwrap_or_default();

            **text = format!(
                "[{}] Pop:{} Brood:{}  |  Food:{:.0}  |  Forage:{} Ret:{} Follow:{}  |  {}  |  Overlay:{}  |  FPS:{:.0}{}\n\
                 WASD:move E:pick Q:drop Shift:trail R:recruit T:dismiss X:swap F:cam/ant Tab:nest",
                mode_str,
                pop, brood,
                colony_food.stored,
                foraging, returning, following,
                clock.speed.label(),
                overlay_label,
                fps,
                player_carrying,
            );
        }
    }
}
