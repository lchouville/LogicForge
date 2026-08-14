use bevy::prelude::*;

use super::appearance::{Appearance, AppearanceLoader, apply_loaded_appearances};
use super::cable::sync_cable_sprite;
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
                    sync_cable_sprite,
                )
                    .chain(),
            );
    }
}
