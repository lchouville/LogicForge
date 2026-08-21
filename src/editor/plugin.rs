use bevy::prelude::*;

use super::camera_control::{
    CameraPanState, PinchState, handle_camera_pan, handle_camera_pinch_zoom,
    handle_camera_wheel_zoom,
};
use super::chip_instance::{
    ChipPickerOpen, handle_chip_picker_row_click, handle_chip_picker_toggle_click,
    spawn_chip_picker, sync_chip_instance_socket_color, sync_chip_picker_collapse,
    sync_chip_picker_rows,
};
use super::chip_structure::{
    ActiveStructureColor, ActiveStructureLabel, ArmedStructureTool, SelectedStructureBlock,
    StructureDragState, StructureLabelFocus, StructurePinLabelFocus,
    handle_delete_selected_structure_block, handle_structure_camera_pan, handle_structure_click,
    handle_structure_click_end, handle_structure_color_button_click, handle_structure_drag,
    handle_structure_label_field_click, handle_structure_label_typing,
    handle_structure_pin_label_field_click, handle_structure_pin_label_suggestion_click,
    handle_structure_pin_label_typing, handle_structure_tool_button_click,
    render_structure_hover_highlight, render_structure_selection_highlight,
    spawn_structure_block_panel, spawn_structure_name_label, spawn_structure_toolbar,
    sync_structure_block_panel, sync_structure_color, sync_structure_label_field_border,
    sync_structure_label_field_text, sync_structure_name_label,
    sync_structure_pin_label_field_border, sync_structure_pin_label_field_text,
    sync_structure_pin_label_suggestions, sync_structure_pin_legs,
    sync_structure_toolbar_highlight, sync_structure_toolbar_visibility,
};
use super::chip_view::{PreChipEditCamera, handle_chip_view_toggle_click};
use super::edit_mode::{
    handle_delete_selected, handle_edit_click_end, handle_edit_click_start, handle_edit_drag,
    handle_selected_rotation, render_hover_highlight, render_selection_highlight, toggle_mode,
};
use super::hud::{
    PointerOverUi, handle_tool_button_click, spawn_action_bar, spawn_toolbar,
    spawn_view_toggle_button, sync_chip_view_toggle_label, sync_mode_button_icon,
    sync_standard_ui_visibility, sync_toolbar_highlight, update_pointer_over_ui,
};
use super::inspector::{spawn_inspector_panel, sync_inspector_panel};
use super::interaction::{
    handle_left_click_end, handle_left_click_start, render_cable_drag_preview,
};
use super::pin_header::{
    PinHeaderLabelFocus, handle_pin_header_label_field_click,
    handle_pin_header_label_suggestion_click, handle_pin_header_label_typing,
    spawn_pin_header_panel, sync_pin_header_label_field_border, sync_pin_header_label_field_text,
    sync_pin_header_label_suggestions, sync_pin_header_panel,
};
use super::placement::{handle_rotation_input, handle_tool_arming};
use super::pointer::{ActiveTouch, PointerState, update_pointer_state};
use super::preview::{sync_placement_preview, tint_placement_preview};
use super::project::{ProjectView, init_project_library};
use super::resources::{
    ArmedTool, EditDragState, InteractionState, Mode, PendingRotation, PickCycleState, Selected,
    SpawnOrderCounter,
};
use super::sidebar::{
    SidebarOpen, handle_project_selection, handle_sidebar_toggle_click, spawn_sidebar,
    sync_project_rows, sync_sidebar_collapse,
};
use crate::rendering::sync::sync_pin_colors;

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
            .init_resource::<Selected>()
            .init_resource::<PointerState>()
            .init_resource::<ActiveTouch>()
            .init_resource::<CameraPanState>()
            .init_resource::<PinchState>()
            .init_resource::<ProjectView>()
            .init_resource::<SidebarOpen>()
            .init_resource::<ArmedStructureTool>()
            .init_resource::<ActiveStructureColor>()
            .init_resource::<SelectedStructureBlock>()
            .init_resource::<StructureDragState>()
            .init_resource::<PreChipEditCamera>()
            .init_resource::<ActiveStructureLabel>()
            .init_resource::<StructureLabelFocus>()
            .init_resource::<StructurePinLabelFocus>()
            .init_resource::<PinHeaderLabelFocus>()
            .init_resource::<ChipPickerOpen>()
            .add_systems(
                Startup,
                (
                    spawn_camera,
                    spawn_toolbar,
                    spawn_action_bar,
                    spawn_view_toggle_button,
                    spawn_inspector_panel,
                    init_project_library,
                    spawn_sidebar,
                    spawn_structure_toolbar,
                    spawn_structure_name_label,
                    spawn_structure_block_panel,
                    spawn_pin_header_panel,
                    spawn_chip_picker,
                ),
            )
            .add_systems(
                Update,
                (
                    // Generic input primitives (cursor position, UI-hover
                    // flag) that both the standard editor and the chip
                    // structure editor need — always on, so it must run
                    // before either of the two mutually-exclusive groups
                    // below regardless of which one is actually active.
                    (update_pointer_state, update_pointer_over_ui),
                    // Wheel/pinch zoom are pure camera math — no dependency
                    // on `GridPosition`/`Cable`/`ArmedTool`/`ProjectView` —
                    // so unlike everything else input-related below, they
                    // run in both the standard editor and the chip structure
                    // editor without needing a view-specific variant.
                    (handle_camera_wheel_zoom, handle_camera_pinch_zoom).chain(),
                    (
                        (
                            handle_tool_arming,
                            handle_rotation_input,
                            toggle_mode,
                            handle_tool_button_click,
                        ),
                        handle_camera_pan,
                        (
                            handle_left_click_start,
                            render_cable_drag_preview,
                            sync_placement_preview,
                            handle_left_click_end,
                            handle_edit_click_start,
                            handle_edit_drag,
                            handle_edit_click_end,
                            handle_delete_selected,
                            handle_selected_rotation,
                        ),
                        (
                            handle_pin_header_label_field_click,
                            handle_pin_header_label_typing,
                            handle_pin_header_label_suggestion_click,
                        ),
                        (
                            handle_chip_picker_toggle_click,
                            handle_chip_picker_row_click,
                        ),
                    )
                        // The whole standard-editor input pipeline is frozen
                        // while the chip structure editor is showing, so a
                        // click there can never reach the interior circuit
                        // sitting far away at `STRUCTURE_SPACE_OFFSET` — see
                        // `chip_view.rs`.
                        .run_if(resource_equals(ProjectView::Standard)),
                    (
                        handle_structure_click,
                        handle_structure_drag,
                        handle_structure_click_end,
                        handle_delete_selected_structure_block,
                        handle_structure_tool_button_click,
                        handle_structure_color_button_click,
                        sync_structure_color,
                        handle_structure_camera_pan,
                        handle_structure_label_field_click,
                        handle_structure_label_typing,
                        handle_structure_pin_label_field_click,
                        handle_structure_pin_label_typing,
                        handle_structure_pin_label_suggestion_click,
                    )
                        .run_if(resource_equals(ProjectView::ChipEdit)),
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    sync_mode_button_icon,
                    sync_toolbar_highlight,
                    sync_inspector_panel,
                    tint_placement_preview,
                    render_selection_highlight,
                    // Reads `PointerState` and `CameraPanState`, written by
                    // `update_pointer_state`/`handle_camera_pan` (and its
                    // chained wheel/pinch zoom) in the other `Update` system
                    // set above — explicit ordering since the two sets
                    // aren't otherwise chained together.
                    render_hover_highlight
                        .after(update_pointer_state)
                        .after(handle_camera_pinch_zoom),
                    sync_standard_ui_visibility,
                    sync_chip_view_toggle_label,
                    sync_structure_toolbar_visibility,
                    sync_structure_toolbar_highlight,
                    sync_structure_label_field_text,
                    sync_structure_label_field_border,
                    (
                        sync_structure_block_panel,
                        sync_structure_pin_label_field_text,
                        sync_structure_pin_label_field_border,
                        sync_structure_pin_label_suggestions,
                    ),
                    (
                        sync_pin_header_panel,
                        sync_pin_header_label_field_text,
                        sync_pin_header_label_field_border,
                        sync_pin_header_label_suggestions,
                    ),
                    (sync_chip_picker_collapse, sync_chip_picker_rows),
                    // Must run after the generic `sync_pin_colors`
                    // (`rendering` plugin) so its lit/unlit tint is the one
                    // that actually ends up on screen — see
                    // `ChipInstanceSocket`'s doc comment.
                    sync_chip_instance_socket_color.after(sync_pin_colors),
                    render_structure_selection_highlight,
                    render_structure_hover_highlight,
                    // Deliberately NOT gated by `run_if(ProjectView::ChipEdit)`
                    // like the structure input-handling systems above: a
                    // project switch (`project::switch_to_project`) despawns
                    // the outgoing project's structure blocks and forces
                    // `ProjectView` back to `Standard` within the same system
                    // call. If this were gated the same way, a switch that
                    // happens to run before this system in the same frame
                    // would flip the view off before the gate is checked,
                    // silently dropping that frame's `RemovedComponents`
                    // event — `RemovedComponents` only buffers for a couple
                    // of frames, so the leg entities for the blocks that just
                    // got despawned would leak as orphaned sprites, stranded
                    // in structure-space until the app restarts. Running
                    // ungated (like the render/sync systems already here)
                    // avoids the race entirely; its own early-return already
                    // makes it a no-op on every frame nothing changed.
                    sync_structure_pin_legs,
                    // Same reasoning as `sync_structure_pin_legs` above — not
                    // respawned per project so there's no leak risk of its
                    // own, but keeping it ungated too avoids depending on
                    // exactly when `ProjectView` flips during a switch.
                    sync_structure_name_label,
                ),
            )
            .add_systems(
                Update,
                (
                    handle_project_selection,
                    sync_project_rows,
                    handle_sidebar_toggle_click,
                    sync_sidebar_collapse,
                    handle_chip_view_toggle_click,
                ),
            );
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
