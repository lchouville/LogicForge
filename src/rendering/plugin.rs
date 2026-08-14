use bevy::prelude::*;

use super::appearance::{apply_loaded_appearances, Appearance, AppearanceLoader};
use super::gizmos::draw_cables;
use super::sync::{sync_lamp_brightness, sync_pin_colors};

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Appearance>()
            .init_asset_loader::<AppearanceLoader>()
            .add_systems(
                Update,
                (
                    apply_loaded_appearances,
                    sync_pin_colors,
                    sync_lamp_brightness,
                    draw_cables,
                )
                    .chain(),
            );
    }
}
