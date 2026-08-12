use bevy::prelude::*;

use crate::constants::EDIT_DRAG_THRESHOLD;
use crate::grid::{cell_to_world, world_to_cell};
use crate::simulation::components::{GridPosition, Pin, Wire};

use super::cursor::cursor_world_position;
use super::hud::PointerOverUi;
use super::resources::{ArmedTool, EditDragState, InteractionState, Mode};
use super::wiring::find_wire_at;

pub fn toggle_mode(
    keys: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<Mode>,
    mut armed: ResMut<ArmedTool>,
    mut interaction: ResMut<InteractionState>,
    mut drag: ResMut<EditDragState>,
) {
    if !keys.just_pressed(KeyCode::Tab) {
        return;
    }
    *mode = match *mode {
        Mode::Interaction => Mode::Edit,
        Mode::Edit => Mode::Interaction,
    };
    armed.0 = None;
    *interaction = InteractionState::Idle;
    *drag = EditDragState::Idle;
}

#[allow(clippy::too_many_arguments)]
pub fn handle_edit_click_start(
    mode: Res<Mode>,
    armed: Res<ArmedTool>,
    pointer_over_ui: Res<PointerOverUi>,
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    mut commands: Commands,
    mut drag: ResMut<EditDragState>,
    positions: Query<(Entity, &GridPosition)>,
    wires: Query<(Entity, &Wire)>,
    pins: Query<&GlobalTransform, With<Pin>>,
) {
    if *mode != Mode::Edit
        || armed.0.is_some()
        || pointer_over_ui.0
        || !buttons.just_pressed(MouseButton::Left)
    {
        return;
    }
    let (camera, camera_transform) = *camera_query;
    let Some(world_pos) = cursor_world_position(&window, camera, camera_transform) else {
        return;
    };

    let cell = world_to_cell(world_pos);
    if let Some((entity, _)) = positions.iter().find(|(_, position)| position.0 == cell) {
        *drag = EditDragState::Pressed {
            entity,
            start_cursor: world_pos,
            dragged: false,
        };
        return;
    }

    // Nothing with a grid position was under the cursor — a wire has no
    // GridPosition of its own, so it gets its own hit-test and, since it
    // can't be moved, deletes immediately on a plain click instead of going
    // through the press/drag/release flow used for components.
    if let Some(wire_entity) = find_wire_at(world_pos, &wires, &pins) {
        commands.entity(wire_entity).despawn();
    }
}

pub fn handle_edit_drag(
    mode: Res<Mode>,
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    mut drag: ResMut<EditDragState>,
    mut positioned: Query<(Entity, &mut GridPosition, &mut Transform)>,
) {
    if *mode != Mode::Edit || !buttons.pressed(MouseButton::Left) {
        return;
    }
    let EditDragState::Pressed {
        entity,
        start_cursor,
        mut dragged,
    } = *drag
    else {
        return;
    };
    let (camera, camera_transform) = *camera_query;
    let Some(world_pos) = cursor_world_position(&window, camera, camera_transform) else {
        return;
    };

    if !dragged && start_cursor.distance(world_pos) > EDIT_DRAG_THRESHOLD {
        dragged = true;
    }

    if dragged {
        let target_cell = world_to_cell(world_pos);
        let blocked = positioned
            .iter()
            .any(|(other, position, _)| other != entity && position.0 == target_cell);
        if !blocked && let Ok((_, mut position, mut transform)) = positioned.get_mut(entity) {
            position.0 = target_cell;
            transform.translation = cell_to_world(target_cell).extend(transform.translation.z);
        }
    }

    *drag = EditDragState::Pressed {
        entity,
        start_cursor,
        dragged,
    };
}

pub fn handle_edit_click_end(
    mode: Res<Mode>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    mut drag: ResMut<EditDragState>,
    children: Query<&Children>,
    pins: Query<Entity, With<Pin>>,
    wires: Query<(Entity, &Wire)>,
) {
    if *mode != Mode::Edit || !buttons.just_released(MouseButton::Left) {
        return;
    }
    let EditDragState::Pressed {
        entity, dragged, ..
    } = *drag
    else {
        return;
    };
    *drag = EditDragState::Idle;

    if dragged {
        // The move was already applied live in `handle_edit_drag`.
        return;
    }

    let doomed_pins: Vec<Entity> = children
        .get(entity)
        .map(|kids| kids.iter().filter(|&child| pins.contains(child)).collect())
        .unwrap_or_default();

    for (wire_entity, wire) in &wires {
        if doomed_pins.contains(&wire.from) || doomed_pins.contains(&wire.to) {
            commands.entity(wire_entity).despawn();
        }
    }

    commands.entity(entity).despawn();
}
