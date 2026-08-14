use bevy::prelude::*;

use super::gizmos::draw_cables;
use super::sync::{sync_lamp_brightness, sync_pin_colors};

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (sync_pin_colors, sync_lamp_brightness, draw_cables));
    }
}
