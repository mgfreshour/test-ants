use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};

use crate::components::ant::{Ant, AntState, CarriedItem, Health, PlayerControlled};
use crate::components::map::MapMarker;
use crate::plugins::ant_ai::ColonyFood;
use crate::plugins::player::{FollowerCount, PlayerMode, RecruitMode};
use crate::plugins::pheromone::{OverlayDisplay, OverlayState};
use crate::resources::active_map::{ActiveMap, MapRegistry};
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
    registry: Res<MapRegistry>,
    active: Res<ActiveMap>,
    stats: Res<ColonyStats>,
    player_mode: Res<PlayerMode>,
    _followers: Res<FollowerCount>,
    recruit_mode: Res<RecruitMode>,
    food_query: Query<&ColonyFood, With<MapMarker>>,
    diagnostics: Res<DiagnosticsStore>,
    ant_query: Query<(&Ant, Option<&CarriedItem>)>,
    player_query: Query<(&Ant, &Health, Option<&CarriedItem>), With<PlayerControlled>>,
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

    // Read food from the player nest map entity.
    let food_stored = food_query.get(registry.player_nest).map_or(0.0, |f| f.stored);

    use crate::components::map::MapKind;
    match active.kind {
        MapKind::Nest { .. } => {
            **text = format!(
                "[NEST] Pop:{} Brood:{}  |  Food:{:.0}  |  {}  |  FPS:{:.0}\n\
                 Tab:surface",
                pop, brood,
                food_stored,
                clock.speed.label(),
                fps,
            );
        }
        MapKind::Surface => {
            let mut foraging = 0u32;
            let mut returning = 0u32;
            let mut following = 0u32;
            let mut attacking = 0u32;
            for (ant, _carried) in &ant_query {
                match ant.state {
                    AntState::Foraging => foraging += 1,
                    AntState::Returning => returning += 1,
                    AntState::Following => following += 1,
                    AntState::Attacking => attacking += 1,
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
                    OverlayDisplay::Recruit => "Recruit",
                    OverlayDisplay::AttackRecruit => "Attack",
                }
            } else {
                "Off"
            };

            let mode_str = if player_mode.controlling { "ANT" } else { "CAM" };

            let player_stats = if let Ok((ant, health, carried)) = player_query.get_single() {
                let carry_str = carried
                    .map(|c| format!(" Carry:{:.0}", c.food_amount))
                    .unwrap_or_default();
                format!(
                    "  |  HP:{:.0}/{:.0} Hunger:{:.0}%{} Followers:{}",
                    health.current, health.max,
                    ant.hunger * 100.0,
                    carry_str,
                    _followers.0,
                )
            } else {
                String::new()
            };

            **text = format!(
                "[{}] Pop:{} Brood:{}  |  Food:{:.0}  |  Forage:{} Ret:{} Follow:{} Atk:{}  |  {}  |  Overlay:{}  |  FPS:{:.0}{}\n\
                 WASD:move E:pick Q:drop Shift:trail R(hold):recruit T:dismiss V:mode({}) X:swap F:feed Tab:nest",
                mode_str,
                pop, brood,
                food_stored,
                foraging, returning, following, attacking,
                clock.speed.label(),
                overlay_label,
                fps,
                player_stats,
                recruit_mode.label(),
            );
        }
        MapKind::SpecialZone { zone_id } => {
            **text = format!(
                "[ZONE {}]  |  {}  |  FPS:{:.0}\n\
                 Tab:return",
                zone_id,
                clock.speed.label(),
                fps,
            );
        }
    }
}
