use bevy::prelude::*;

use crate::constants::{COLOR_NEUTRAL, SPAWN_Z_STEP};
use crate::grid::{cell_to_world, world_to_cell};
use crate::simulation::components::{GridPosition, SignalValue, Switch};

use super::hud::PointerOverUi;
use super::placement::{pick_entity_at_cell, place_tool};
use super::pointer::PointerState;
use super::project::ProjectLibrary;
use super::resources::{
    ArmedTool, ChipInstanceSlotAllocator, InteractionState, Mode, PendingRotation, PickCycleState,
    SpawnOrderCounter, ToolKind,
};
use crate::rendering::cable::spawn_cable;

#[allow(clippy::too_many_arguments)]
pub fn handle_left_click_start(
    pointer: Res<PointerState>,
    mode: Res<Mode>,
    pointer_over_ui: Res<PointerOverUi>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut armed: ResMut<ArmedTool>,
    mut rotation: ResMut<PendingRotation>,
    mut interaction: ResMut<InteractionState>,
    mut spawn_order: ResMut<SpawnOrderCounter>,
    mut cycle: ResMut<PickCycleState>,
    library: Res<ProjectLibrary>,
    mut chip_slots: ResMut<ChipInstanceSlotAllocator>,
    mut switches: Query<(Entity, &GridPosition, &mut Switch, &Children)>,
    mut signals: Query<&mut SignalValue>,
) {
    if pointer_over_ui.0 || !pointer.just_pressed {
        return;
    }

    let Some(world_pos) = pointer.world_pos else {
        return;
    };

    if let Some(tool) = armed.0 {
        let cell = world_to_cell(world_pos);
        if tool == ToolKind::Cable {
            // Cable placement needs a start *and* end cell, so it doesn't
            // place immediately like the other tools — it waits for
            // `handle_left_click_end` to supply the end and disarm.
            *interaction = InteractionState::PlacingCable { start_cell: cell };
        } else {
            let z = spawn_order.0;
            spawn_order.0 += SPAWN_Z_STEP;
            place_tool(
                &mut commands,
                &asset_server,
                &library,
                tool,
                cell,
                rotation.0,
                z,
                &mut chip_slots,
            );
            armed.0 = None;
            rotation.0 = 0;
        }
        return;
    }

    // Toggling only applies in Interaction mode; Edit mode's own click
    // handlers own move/delete instead.
    if *mode != Mode::Interaction {
        return;
    }

    let cell = world_to_cell(world_pos);
    let candidates: Vec<Entity> = switches
        .iter()
        .filter(|(_, position, _, _)| position.0 == cell)
        .map(|(entity, _, _, _)| entity)
        .collect();
    let Some(target) = pick_entity_at_cell(cell, candidates, &mut cycle) else {
        return;
    };
    let Ok((_, _, mut switch, children)) = switches.get_mut(target) else {
        return;
    };
    switch.on = !switch.on;
    let value = if switch.on { 1.0 } else { 0.0 };
    for child in children.iter() {
        if let Ok(mut signal) = signals.get_mut(child) {
            signal.0 = value;
        }
    }
}

pub fn render_cable_drag_preview(
    interaction: Res<InteractionState>,
    pointer: Res<PointerState>,
    mut gizmos: Gizmos,
) {
    let InteractionState::PlacingCable { start_cell } = *interaction else {
        return;
    };
    let Some(cursor_world) = pointer.world_pos else {
        return;
    };
    gizmos.line_2d(cell_to_world(start_cell), cursor_world, COLOR_NEUTRAL);
}

pub fn handle_left_click_end(
    pointer: Res<PointerState>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut armed: ResMut<ArmedTool>,
    mut interaction: ResMut<InteractionState>,
) {
    if !pointer.just_released {
        return;
    }
    let InteractionState::PlacingCable { start_cell } = *interaction else {
        return;
    };
    *interaction = InteractionState::Idle;
    armed.0 = None;

    let Some(world_pos) = pointer.world_pos else {
        return;
    };
    let end_cell = world_to_cell(world_pos);
    if end_cell != start_cell {
        spawn_cable(&mut commands, &asset_server, start_cell, end_cell);
    }
}
