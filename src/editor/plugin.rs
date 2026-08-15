use bevy::prelude::*;

use super::edit_mode::{
    handle_edit_click_end, handle_edit_click_start, handle_edit_drag, toggle_mode,
};
use super::hud::{
    PointerOverUi, handle_tool_button_click, spawn_mode_label, spawn_toolbar, sync_mode_label,
    sync_toolbar_highlight, update_pointer_over_ui,
};
use super::interaction::{
    handle_left_click_end, handle_left_click_start, render_cable_drag_preview,
};
use super::placement::{handle_rotation_input, handle_tool_arming};
use super::preview::{sync_placement_preview, tint_placement_preview};
use super::resources::{
    ArmedTool, EditDragState, InteractionState, Mode, PendingRotation, PickCycleState,
    SpawnOrderCounter,
};

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArmedTool>()
            .init_resource::<PendingRotation>()
            .init_resource::<InteractionState>()
            .init_resource::<Mode>()
            .init_resource::<EditDragState>()
            .init_resource::<PointerOverUi>()
            .init_resource::<PickCycleState>()
            .init_resource::<SpawnOrderCounter>()
            .add_systems(Startup, (spawn_camera, spawn_mode_label, spawn_toolbar))
            .add_systems(
                Update,
                (
                    (
                        handle_tool_arming,
                        handle_rotation_input,
                        toggle_mode,
                        handle_tool_button_click,
                        update_pointer_over_ui,
                    ),
                    (
                        handle_left_click_start,
                        render_cable_drag_preview,
                        sync_placement_preview,
                        handle_left_click_end,
                        handle_edit_click_start,
                        handle_edit_drag,
                        handle_edit_click_end,
                    ),
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    sync_mode_label,
                    sync_toolbar_highlight,
                    tint_placement_preview,
                ),
            );
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
