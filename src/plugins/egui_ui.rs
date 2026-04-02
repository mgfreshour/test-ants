use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use rand::Rng;

use crate::components::ant::{Ant, AntState, CarriedItem, ColonyMember, Health, PlayerControlled};
use crate::components::map::{MapKind, MapMarker};
use crate::components::nest::NestTask;
use crate::components::terrain::FoodSource;
use crate::plugins::ant_ai::ColonyFood;
use crate::plugins::combat::{GameResult, Spider};
use crate::plugins::pheromone::{OverlayDisplay, OverlayState};
use crate::plugins::nest_pheromone::NestPheromoneOverlayState;
use crate::plugins::player::{
    ActionContext, FollowerCount, PlayerAction, PlayerMode, RecruitMode, ToastQueue,
};
use crate::resources::active_map::{ActiveMap, MapRegistry};
use crate::resources::colony::{AggressionSettings, BehaviorSliders, CasteRatios, ColonyStats};
use crate::resources::simulation::{SimClock, SimConfig, SimSpeed};

pub struct EguiUiPlugin;

/// Whether the colony management panel is visible.
#[derive(Resource)]
pub struct PanelVisible(pub bool);

impl Default for PanelVisible {
    fn default() -> Self {
        Self(true)
    }
}

/// Whether the keyboard shortcut overlay is visible.
#[derive(Resource, Default)]
pub struct ShortcutOverlayVisible(pub bool);

impl Plugin for EguiUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .init_resource::<PanelVisible>()
            .init_resource::<ShortcutOverlayVisible>()
            .init_resource::<AggressionSettings>()
            .init_resource::<CasteRatios>()
            .add_systems(Update, (toggle_panel, toggle_shortcut_overlay, escape_close_overlays))
            .add_systems(
                EguiPrimaryContextPass,
                (
                    colony_management_panel,
                    player_hud_panel,
                    minimap_panel,
                    toast_display,
                    shortcut_overlay_panel,
                    fps_display,
                    game_result_overlay,
                ),
            );
    }
}

/// Backtick toggles panel visibility.
fn toggle_panel(input: Res<ButtonInput<KeyCode>>, mut visible: ResMut<PanelVisible>) {
    if input.just_pressed(KeyCode::Backquote) {
        visible.0 = !visible.0;
    }
}

/// ? key toggles shortcut overlay.
fn toggle_shortcut_overlay(
    input: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<ShortcutOverlayVisible>,
) {
    if input.just_pressed(KeyCode::Slash)
        && (input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight))
    {
        visible.0 = !visible.0;
    }
}

/// Escape closes all overlays.
fn escape_close_overlays(
    input: Res<ButtonInput<KeyCode>>,
    mut panel: ResMut<PanelVisible>,
    mut shortcut: ResMut<ShortcutOverlayVisible>,
) {
    if input.just_pressed(KeyCode::Escape) {
        if shortcut.0 {
            shortcut.0 = false;
        } else if panel.0 {
            panel.0 = false;
        }
    }
}

/// Main colony management panel rendered via egui.
#[allow(clippy::too_many_arguments)]
fn colony_management_panel(
    mut contexts: EguiContexts,
    mut clock: ResMut<SimClock>,
    mut panel_visible: ResMut<PanelVisible>,
    active: Res<ActiveMap>,
    registry: Res<MapRegistry>,
    stats: Res<ColonyStats>,
    mut caste_ratios: ResMut<CasteRatios>,
    mut aggression: ResMut<AggressionSettings>,
    mut overlay_state: ResMut<OverlayState>,
    mut nest_overlay_state: ResMut<NestPheromoneOverlayState>,
    mut env: ResMut<crate::plugins::environment::EnvironmentState>,
    food_query: Query<&ColonyFood, With<MapMarker>>,
    mut sliders_query: Query<&mut BehaviorSliders, With<MapMarker>>,
    underground_count: Query<&NestTask>,
    ant_query: Query<&ColonyMember, With<Ant>>,
) {
    if !panel_visible.0 {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::SidePanel::left("colony_panel")
        .default_width(220.0)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Colony");
            ui.separator();

            // -- Sim Controls
            ui.horizontal(|ui| {
                ui.label("Speed:");
                let speeds = [
                    (SimSpeed::Paused, "||"),
                    (SimSpeed::Normal, "1x"),
                    (SimSpeed::Fast, "2x"),
                    (SimSpeed::VeryFast, "4x"),
                    (SimSpeed::Ultra, "8x"),
                ];
                for (speed, label) in &speeds {
                    let selected = clock.speed == *speed;
                    if ui.selectable_label(selected, *label).clicked() {
                        clock.speed = *speed;
                    }
                }
            });

            ui.label(format!("Time: {:.0}s", clock.elapsed));
            ui.separator();

            // -- View
            let view_label = match active.kind {
                MapKind::Surface => "Surface",
                MapKind::Nest { .. } => "Underground",
                MapKind::SpecialZone { .. } => "Zone",
            };
            ui.label(format!("View: {}  [Tab]", view_label));
            ui.separator();

            // -- Colony Stats
            ui.collapsing("Stats", |ui| {
                let pop = stats.workers + stats.soldiers + stats.drones;
                let brood = stats.eggs + stats.larvae + stats.pupae;
                let food_stored = food_query.get(registry.player_nest).map_or(0.0, |f| f.stored);
                let underground = underground_count.iter().count();
                let surface = ant_query.iter().filter(|c| c.colony_id == 0).count().saturating_sub(underground);

                ui.label(format!("Population: {}", pop));
                ui.label(format!("  Workers: {}", stats.workers));
                ui.label(format!("  Soldiers: {}", stats.soldiers));
                ui.label(format!("  Drones: {}", stats.drones));
                ui.label(format!("Brood: {}", brood));
                ui.label(format!("  Eggs: {}", stats.eggs));
                ui.label(format!("  Larvae: {}", stats.larvae));
                ui.label(format!("  Pupae: {}", stats.pupae));
                ui.label(format!("Food: {:.0}", food_stored));
                ui.label(format!("Surface: {}  Underground: {}", surface, underground));
            });
            ui.separator();

            // -- Job Distribution
            if let Ok(mut sliders) = sliders_query.get_mut(registry.player_nest) {
                ui.collapsing("Job Distribution", |ui| {
                    let mut changed = false;
                    let mut forage = sliders.forage;
                    let mut nurse = sliders.nurse;
                    let mut dig = sliders.dig;
                    let mut defend = sliders.defend;

                    changed |= ui.add(egui::Slider::new(&mut forage, 0.0..=1.0).text("Forage"))
                        .on_hover_text("% of surface ants dedicated to finding food")
                        .changed();
                    changed |= ui.add(egui::Slider::new(&mut nurse, 0.0..=1.0).text("Nurse"))
                        .on_hover_text("% of ants assigned to feed larvae underground")
                        .changed();
                    changed |= ui.add(egui::Slider::new(&mut dig, 0.0..=1.0).text("Dig"))
                        .on_hover_text("% of ants assigned to excavate new tunnels")
                        .changed();
                    changed |= ui.add(egui::Slider::new(&mut defend, 0.0..=1.0).text("Defend"))
                        .on_hover_text("% of ants assigned to patrol and defend")
                        .changed();

                    if changed {
                        let sum = forage + nurse + dig + defend;
                        if sum > 0.0 {
                            sliders.forage = forage / sum;
                            sliders.nurse = nurse / sum;
                            sliders.dig = dig / sum;
                            sliders.defend = defend / sum;
                        }
                    }
                });
                ui.separator();
            }

            // -- Caste Birthrates
            ui.collapsing("Caste Birthrates", |ui| {
                let mut worker = caste_ratios.worker;
                let mut soldier = caste_ratios.soldier;
                let mut drone = caste_ratios.drone;
                let mut changed = false;

                changed |= ui.add(egui::Slider::new(&mut worker, 0.0..=1.0).text("Worker"))
                    .on_hover_text("Fraction of new ants born as workers")
                    .changed();
                changed |= ui.add(egui::Slider::new(&mut soldier, 0.0..=1.0).text("Soldier"))
                    .on_hover_text("Fraction of new ants born as soldiers")
                    .changed();
                changed |= ui.add(egui::Slider::new(&mut drone, 0.0..=1.0).text("Drone"))
                    .on_hover_text("Fraction of new ants born as drones")
                    .changed();

                if changed {
                    let sum = worker + soldier + drone;
                    if sum > 0.0 {
                        caste_ratios.worker = worker / sum;
                        caste_ratios.soldier = soldier / sum;
                        caste_ratios.drone = drone / sum;
                    }
                }
            });
            ui.separator();

            // -- Aggression
            ui.collapsing("Aggression", |ui| {
                ui.add(egui::Slider::new(&mut aggression.patrol_radius, 50.0..=500.0).text("Patrol Radius"))
                    .on_hover_text("How far defenders roam from the nest entrance");
                ui.add(egui::Slider::new(&mut aggression.alarm_threshold, 0.1..=5.0).text("Alarm Threshold"))
                    .on_hover_text("Pheromone intensity required to trigger defender response");
            });
            ui.separator();

            // -- Environment Controls
            ui.collapsing("Environment", |ui| {
                ui.label(format!("Time of Day: {:.1}h", env.time_of_day * 24.0));

                if env.is_raining {
                    ui.colored_label(egui::Color32::from_rgb(100, 150, 255), "🌧 RAINING");
                } else {
                    ui.colored_label(egui::Color32::from_rgb(200, 200, 100), "☀ Clear");
                }

                if env.flood_level > 0.0 {
                    ui.colored_label(
                        egui::Color32::from_rgb(50, 100, 200),
                        format!("Flood Level: {:.1}%", env.flood_level * 100.0)
                    );
                }

                ui.separator();
                ui.label("Trigger Event:");
                ui.horizontal(|ui| {
                    if ui.button("☔ Rain").clicked() {
                        env.manual_triggers.push(crate::plugins::environment::HazardTrigger::Rain);
                    }
                    if ui.button("👣 Footstep").clicked() {
                        env.manual_triggers.push(crate::plugins::environment::HazardTrigger::Footstep);
                    }
                    if ui.button("🔪 Mower").clicked() {
                        env.manual_triggers.push(crate::plugins::environment::HazardTrigger::Lawnmower);
                    }
                    if ui.button("☠️ Spray").clicked() {
                        env.manual_triggers.push(crate::plugins::environment::HazardTrigger::Pesticide);
                    }
                });
            });
            ui.separator();

            // -- Overlay Controls
            match active.kind {
                MapKind::Surface => {
                    ui.collapsing("Overlay [H]", |ui| {
                        let mut vis = overlay_state.visible;
                        if ui.checkbox(&mut vis, "Show overlay").changed() {
                            overlay_state.visible = vis;
                        }
                        if overlay_state.visible {
                            let displays = [
                                (OverlayDisplay::All, "All"),
                                (OverlayDisplay::Home, "Home"),
                                (OverlayDisplay::Food, "Food"),
                                (OverlayDisplay::Alarm, "Alarm"),
                                (OverlayDisplay::Trail, "Trail"),
                                (OverlayDisplay::Recruit, "Recruit"),
                                (OverlayDisplay::AttackRecruit, "Attack"),
                            ];
                            for (display, label) in &displays {
                                let selected = overlay_state.display_type == *display;
                                if ui.selectable_label(selected, *label).clicked() {
                                    overlay_state.display_type = *display;
                                }
                            }
                        }
                    });
                }
                MapKind::Nest { .. } => {
                    ui.collapsing("Nest Overlay [N]", |ui| {
                        let mut vis = nest_overlay_state.visible;
                        if ui.checkbox(&mut vis, "Show overlay").changed() {
                            nest_overlay_state.visible = vis;
                        }
                    });
                }
                _ => {}
            }

            // -- Collapse button
            ui.separator();
            if ui.small_button("Hide [`]").clicked() {
                panel_visible.0 = false;
            }
        });
}

// ── Player HUD & Action Bar ────────────────────────────────────────

/// Bottom-center player HUD with health, hunger, carried item, followers, and action buttons.
#[allow(clippy::too_many_arguments)]
fn player_hud_panel(
    mut contexts: EguiContexts,
    action_ctx: Res<ActionContext>,
    player_mode: Res<PlayerMode>,
    followers: Res<FollowerCount>,
    recruit_mode: Res<RecruitMode>,
    mut action_writer: MessageWriter<PlayerAction>,
    player_query: Query<(&Ant, &Health, Option<&CarriedItem>), With<PlayerControlled>>,
) {
    if !player_mode.controlling {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::TopBottomPanel::bottom("player_hud")
        .max_height(80.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // -- Player stats
                if let Ok((ant, health, carried)) = player_query.single() {
                    // HP bar
                    let hp_frac = (health.current / health.max).clamp(0.0, 1.0);
                    let hp_color = if hp_frac > 0.5 {
                        egui::Color32::from_rgb(80, 200, 80)
                    } else if hp_frac > 0.25 {
                        egui::Color32::from_rgb(200, 200, 40)
                    } else {
                        egui::Color32::from_rgb(200, 50, 50)
                    };
                    ui.vertical(|ui| {
                        ui.label(format!("HP: {:.0}/{:.0}", health.current, health.max));
                        let bar = egui::ProgressBar::new(hp_frac)
                            .fill(hp_color)
                            .desired_width(100.0);
                        ui.add(bar);
                    });

                    ui.separator();

                    // Hunger bar
                    let hunger_frac = ant.hunger.clamp(0.0, 1.0);
                    ui.vertical(|ui| {
                        ui.label(format!("Hunger: {:.0}%", hunger_frac * 100.0));
                        let bar = egui::ProgressBar::new(hunger_frac)
                            .fill(egui::Color32::from_rgb(200, 140, 40))
                            .desired_width(80.0);
                        ui.add(bar);
                    });

                    ui.separator();

                    // Carried item
                    if let Some(item) = carried {
                        ui.label(format!("Carry: {:.0}", item.food_amount))
                            .on_hover_text("Amount of food being carried");
                    } else {
                        ui.weak("Empty");
                    }

                    ui.separator();

                    // Follower count
                    ui.label(format!("Followers: {}", followers.0))
                        .on_hover_text("Number of ants following you");

                    ui.separator();

                    // Mode indicator
                    let mode_label = match *recruit_mode {
                        RecruitMode::Follow => "Follow",
                        RecruitMode::Attack => "Attack",
                    };
                    ui.label(format!("Mode: {} [V]", mode_label))
                        .on_hover_text("Recruit mode — V to toggle");
                } else {
                    ui.label("No ant controlled");
                }

                ui.separator();

                // -- Action buttons
                ui.horizontal(|ui| {
                    let btn = |ui: &mut egui::Ui, label: &str, hotkey: &str, enabled: bool, tooltip: &str| -> bool {
                        let text = format!("{} ({})", label, hotkey);
                        let response = ui.add_enabled(enabled, egui::Button::new(&text));
                        let clicked = response.clicked();
                        response.on_hover_text(tooltip);
                        clicked
                    };

                    if btn(ui, "Pick Up", "E", action_ctx.can_pickup, "Pick up nearby food") {
                        action_writer.write(PlayerAction::Pickup);
                    }
                    if btn(ui, "Drop", "Q", action_ctx.can_drop, "Drop carried item") {
                        action_writer.write(PlayerAction::Drop);
                    }
                    if btn(ui, "Feed", "F", action_ctx.can_feed, "Share food with nearby ant") {
                        action_writer.write(PlayerAction::Feed);
                    }

                    // Trail button shows active state
                    let trail_label = if action_ctx.trail_active { "Trail *" } else { "Trail" };
                    ui.add_enabled(true, egui::Button::new(format!("{} (Shift)", trail_label)))
                        .on_hover_text("Hold Shift while moving to lay pheromone trail");

                    if btn(ui, "Recruit", "R", action_ctx.can_recruit, "Hold to recruit nearby ants") {
                        action_writer.write(PlayerAction::Recruit);
                    }
                    if btn(ui, "Dismiss", "T", action_ctx.can_dismiss, "Dismiss all followers") {
                        action_writer.write(PlayerAction::Dismiss);
                    }
                    if btn(ui, "Swap", "X", action_ctx.can_swap, "Swap control to nearest ant") {
                        action_writer.write(PlayerAction::Swap);
                    }
                    if btn(ui, "Attack", "Space", action_ctx.can_attack, "Attack nearest enemy") {
                        action_writer.write(PlayerAction::Attack);
                    }
                });
            });
        });
}

// ── Minimap ────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn minimap_panel(
    mut contexts: EguiContexts,
    active: Res<ActiveMap>,
    config: Res<SimConfig>,
    env: Res<crate::plugins::environment::EnvironmentState>,
    ant_query: Query<(&Transform, &ColonyMember), With<Ant>>,
    food_query: Query<(&Transform, &FoodSource)>,
    spider_query: Query<&Transform, With<Spider>>,
) {
    if !matches!(active.kind, MapKind::Surface) {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };

    let minimap_size = 140.0;
    let scale_x = minimap_size / config.world_width;
    let scale_y = minimap_size / config.world_height;

    egui::Window::new("Minimap")
        .anchor(egui::Align2::RIGHT_TOP, [-8.0, 8.0])
        .resizable(false)
        .collapsible(false)
        .title_bar(false)
        .fixed_size([minimap_size, minimap_size])
        .show(ctx, |ui| {
            let (response, painter) = ui.allocate_painter(
                egui::vec2(minimap_size, minimap_size),
                egui::Sense::click(),
            );

            let rect = response.rect;

            // Background — green grass, darkened at night, blue-shifted in rain
            let dist_from_noon = (env.time_of_day - 0.5).abs() * 2.0;
            let night_factor = dist_from_noon.clamp(0.0, 1.0);
            let base_r = (55.0 * (1.0 - night_factor * 0.6)) as u8;
            let base_g = (110.0 * (1.0 - night_factor * 0.6)) as u8;
            let base_b = (40.0 * (1.0 - night_factor * 0.5)) as u8;
            let (bg_r, bg_g, bg_b) = if env.is_raining {
                // Shift towards blue-grey when raining
                (base_r.saturating_sub(10), base_g.saturating_sub(10), base_b.saturating_add(30))
            } else {
                (base_r, base_g, base_b)
            };
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(bg_r, bg_g, bg_b));

            // Nest marker
            let nest_x = rect.left() + config.nest_position.x * scale_x;
            let nest_y = rect.bottom() - config.nest_position.y * scale_y;
            painter.circle_filled(
                egui::pos2(nest_x, nest_y),
                3.0,
                egui::Color32::from_rgb(200, 200, 60),
            );

            // Food sources
            for (tf, food) in &food_query {
                if food.remaining <= 0.0 {
                    continue;
                }
                let x = rect.left() + tf.translation.x * scale_x;
                let y = rect.bottom() - tf.translation.y * scale_y;
                painter.circle_filled(
                    egui::pos2(x, y),
                    2.0,
                    egui::Color32::from_rgb(50, 180, 50),
                );
            }

            // Spiders
            for tf in &spider_query {
                let x = rect.left() + tf.translation.x * scale_x;
                let y = rect.bottom() - tf.translation.y * scale_y;
                painter.circle_filled(
                    egui::pos2(x, y),
                    2.5,
                    egui::Color32::from_rgb(120, 80, 40),
                );
            }

            // Hazard zones
            for (_id, hazard) in &env.active_hazards {
                use crate::plugins::environment::HazardKind;
                let hx = rect.left() + hazard.position.x * scale_x;
                let hy = rect.bottom() - hazard.position.y * scale_y;
                let life_frac = (hazard.remaining_time / hazard.max_time).clamp(0.0, 1.0);

                let (color, half_w, half_h) = match hazard.kind {
                    HazardKind::Footstep => (
                        egui::Color32::from_rgba_unmultiplied(80, 60, 40, (160.0 * life_frac) as u8),
                        hazard.radius * scale_x,
                        hazard.radius * 1.25 * scale_y,
                    ),
                    HazardKind::Lawnmower => (
                        egui::Color32::from_rgba_unmultiplied(220, 50, 30, 180),
                        minimap_size / 2.0, // spans most of the width
                        hazard.radius * scale_y,
                    ),
                    HazardKind::Pesticide => (
                        egui::Color32::from_rgba_unmultiplied(120, 180, 30, 140),
                        hazard.radius * scale_x,
                        hazard.radius * scale_y,
                    ),
                };

                let hazard_rect = egui::Rect::from_center_size(
                    egui::pos2(hx, hy),
                    egui::vec2(half_w * 2.0, half_h * 2.0),
                );
                painter.rect_filled(hazard_rect, 1.0, color);
            }

            // Rain dots on minimap
            if env.is_raining {
                let mut rng = rand::thread_rng();
                for _ in 0..6 {
                    let rx = rect.left() + rng.gen_range(0.0..minimap_size);
                    let ry = rect.top() + rng.gen_range(0.0..minimap_size);
                    painter.circle_filled(
                        egui::pos2(rx, ry),
                        1.0,
                        egui::Color32::from_rgba_unmultiplied(100, 150, 255, 120),
                    );
                }
            }

            // Ants (sample to avoid performance issues)
            let mut player_count = 0u32;
            let mut enemy_count = 0u32;
            for (tf, colony) in &ant_query {
                let x = rect.left() + tf.translation.x * scale_x;
                let y = rect.bottom() - tf.translation.y * scale_y;
                let color = if colony.colony_id == 0 {
                    player_count += 1;
                    if player_count % 3 != 0 { continue; } // sample every 3rd
                    egui::Color32::from_rgb(40, 40, 40)
                } else {
                    enemy_count += 1;
                    if enemy_count % 3 != 0 { continue; }
                    egui::Color32::from_rgb(200, 40, 40)
                };
                painter.circle_filled(egui::pos2(x, y), 1.0, color);
            }
        });
}

// ── Toast Notifications ────────────────────────────────────────────

fn toast_display(
    mut contexts: EguiContexts,
    toasts: Res<ToastQueue>,
) {
    if toasts.toasts.is_empty() {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::Area::new(egui::Id::new("toast_area"))
        .anchor(egui::Align2::CENTER_TOP, [0.0, 40.0])
        .show(ctx, |ui| {
            for toast in &toasts.toasts {
                let alpha = (toast.timer.min(1.0) * 255.0) as u8;
                let frame = egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_unmultiplied(30, 30, 30, alpha.min(220)))
                    .inner_margin(egui::Margin::same(8))
                    .corner_radius(4.0);
                frame.show(ui, |ui| {
                    ui.colored_label(
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
                        &toast.message,
                    );
                });
                ui.add_space(4.0);
            }
        });
}

// ── Keyboard Shortcut Reference ────────────────────────────────────

fn shortcut_overlay_panel(
    mut contexts: EguiContexts,
    visible: Res<ShortcutOverlayVisible>,
) {
    if !visible.0 {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::Window::new("Keyboard Shortcuts")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.heading("Movement");
            ui.label("WASD / Arrows — Move ant");
            ui.label("Enter — Enter/exit nest portal");
            ui.label("Tab — Switch surface/underground view");
            ui.separator();

            ui.heading("Actions");
            ui.label("E — Pick up food");
            ui.label("Q — Drop carried item");
            ui.label("F — Feed nearby ant");
            ui.label("Space — Attack nearest enemy");
            ui.separator();

            ui.heading("Pheromones & Followers");
            ui.label("Shift (hold) — Lay trail pheromone");
            ui.label("R (hold) — Recruit nearby ants");
            ui.label("T — Dismiss all followers");
            ui.label("V — Toggle recruit mode (Follow/Attack)");
            ui.separator();

            ui.heading("Camera & UI");
            ui.label("G — Toggle ant control on/off");
            ui.label("X — Swap to nearest ant");
            ui.label("H — Toggle pheromone overlay (surface)");
            ui.label("N — Toggle pheromone overlay (nest)");
            ui.label("` — Toggle colony panel");
            ui.label("? — This shortcut reference");
            ui.label("Esc — Close overlay/panel");
            ui.separator();

            ui.heading("Simulation");
            ui.label("Pause/speed controls in colony panel");
            ui.separator();

            if ui.button("Close [?]").clicked() {
                // Will be closed next frame by toggle
            }
        });
}

// ── FPS Display ────────────────────────────────────────────────────

fn fps_display(
    mut contexts: EguiContexts,
    diagnostics: Res<DiagnosticsStore>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return };

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    egui::Area::new(egui::Id::new("fps_counter"))
        .anchor(egui::Align2::RIGHT_BOTTOM, [-8.0, -8.0])
        .show(ctx, |ui| {
            ui.weak(format!("FPS: {:.0}", fps));
        });
}

// ── Game Result Overlay ────────────────────────────────────────────

fn game_result_overlay(
    mut contexts: EguiContexts,
    result: Res<GameResult>,
) {
    if !result.decided {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else { return };

    egui::Window::new("Game Over")
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            if result.player_won {
                ui.heading("VICTORY!");
                ui.label("The enemy colony has been destroyed.");
            } else {
                ui.heading("DEFEAT");
                ui.label("Your colony has been destroyed.");
            }
        });
}
