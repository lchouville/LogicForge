use bevy::prelude::*;

use super::appearance::{Appearance, AppearanceLoader, apply_loaded_appearances};
use super::background_grid::sync_background_grid;
use super::cable::{rebuild_cable_segments, sync_cable_sprite};
use super::sync::{sync_lamp_brightness, sync_pin_colors};
use crate::editor::camera_control::handle_camera_pinch_zoom;

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Appearance>()
            .init_asset_loader::<AppearanceLoader>()
            .add_systems(
                Update,
                (
                    rebuild_cable_segments,
                    // `.after(...)` rather than folding into the chain below:
                    // the background grid reacts to the camera moving (see
                    // `sync_background_grid`'s doc comment), so it needs to
                    // run after the editor's camera pan/wheel/pinch systems
                    // have applied this frame's movement — otherwise the
                    // tile pool would lag a frame behind during a pan/zoom.
                    sync_background_grid.after(handle_camera_pinch_zoom),
                    apply_loaded_appearances,
                    sync_pin_colors,
                    sync_lamp_brightness,
                    sync_cable_sprite,
                )
                    .chain(),
            );
    }
}
