use bevy::prelude::*;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;

/// HudPlugin now only registers the FrameTimeDiagnosticsPlugin.
/// All HUD rendering has moved to `plugins/egui_ui.rs`.
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default());
    }
}
