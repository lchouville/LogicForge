use bevy::prelude::*;

use crate::constants::EDIT_DRAG_THRESHOLD;
use crate::grid::{cell_to_world, world_to_cell};
use crate::simulation::components::{Cable, GridPosition};

use super::cursor::cursor_world_position;
use super::hud::PointerOverUi;
use super::placement::pick_entity_at_cell;
use super::resources::{ArmedTool, EditDragState, InteractionState, Mode, PickCycleState};
use super::wiring::{CableEnd, CableHit, find_cable_at};

pub fn toggle_mode(
    keys: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<Mode>,
    mut armed: ResMut<ArmedTool>,
    mut interaction: ResMut<InteractionState>,
    mut drag: ResMut<EditDragState>,
    mut cycle: ResMut<PickCycleState>,
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
    *cycle = PickCycleState::default();
}

#[allow(clippy::too_many_arguments)]
pub fn handle_edit_click_start(
    mode: Res<Mode>,
    armed: Res<ArmedTool>,
    pointer_over_ui: Res<PointerOverUi>,
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    mut drag: ResMut<EditDragState>,
    mut cycle: ResMut<PickCycleState>,
    positions: Query<(Entity, &GridPosition)>,
    cables: Query<(Entity, &Cable)>,
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
    let candidates: Vec<Entity> = positions
        .iter()
        .filter(|(_, position)| position.0 == cell)
        .map(|(entity, _)| entity)
        .collect();
    if let Some(entity) = pick_entity_at_cell(cell, candidates, &mut cycle) {
        *drag = EditDragState::Pressed {
            entity,
            start_cursor: world_pos,
            dragged: false,
        };
        return;
    }

    // Nothing with a GridPosition was under the cursor — try a cable
    // instead, which has no GridPosition of its own and gets its own
    // endpoint-vs-body hit-test.
    let Some((entity, hit)) = find_cable_at(world_pos, &cables) else {
        return;
    };
    let Ok((_, cable)) = cables.get(entity) else {
        return;
    };
    *drag = match hit {
        CableHit::Endpoint(which) => EditDragState::CableEndpoint {
            entity,
            which,
            start_cursor: world_pos,
            dragged: false,
        },
        CableHit::Body => EditDragState::CableBody {
            entity,
            start_cursor: world_pos,
            orig_start: cable.start,
            orig_end: cable.end,
            dragged: false,
        },
    };
}

pub fn handle_edit_drag(
    mode: Res<Mode>,
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    mut drag: ResMut<EditDragState>,
    mut positioned: Query<(&mut GridPosition, &mut Transform)>,
    mut cables: Query<&mut Cable>,
) {
    if *mode != Mode::Edit || !buttons.pressed(MouseButton::Left) {
        return;
    }
    let (camera, camera_transform) = *camera_query;
    let Some(world_pos) = cursor_world_position(&window, camera, camera_transform) else {
        return;
    };

    match *drag {
        EditDragState::Idle => {}
        EditDragState::Pressed {
            entity,
            start_cursor,
            mut dragged,
        } => {
            if !dragged && start_cursor.distance(world_pos) > EDIT_DRAG_THRESHOLD {
                dragged = true;
            }
            if dragged
                && let Ok((mut position, mut transform)) = positioned.get_mut(entity)
            {
                let target_cell = world_to_cell(world_pos);
                position.0 = target_cell;
                transform.translation = cell_to_world(target_cell).extend(transform.translation.z);
            }
            *drag = EditDragState::Pressed {
                entity,
                start_cursor,
                dragged,
            };
        }
        EditDragState::CableBody {
            entity,
            start_cursor,
            orig_start,
            orig_end,
            mut dragged,
        } => {
            if !dragged && start_cursor.distance(world_pos) > EDIT_DRAG_THRESHOLD {
                dragged = true;
            }
            if dragged
                && let Ok(mut cable) = cables.get_mut(entity)
            {
                let delta = world_to_cell(world_pos) - world_to_cell(start_cursor);
                cable.start = orig_start + delta;
                cable.end = orig_end + delta;
            }
            *drag = EditDragState::CableBody {
                entity,
                start_cursor,
                orig_start,
                orig_end,
                dragged,
            };
        }
        EditDragState::CableEndpoint {
            entity,
            which,
            start_cursor,
            mut dragged,
        } => {
            if !dragged && start_cursor.distance(world_pos) > EDIT_DRAG_THRESHOLD {
                dragged = true;
            }
            if dragged
                && let Ok(mut cable) = cables.get_mut(entity)
            {
                let target_cell = world_to_cell(world_pos);
                match which {
                    CableEnd::Start => cable.start = target_cell,
                    CableEnd::End => cable.end = target_cell,
                }
            }
            *drag = EditDragState::CableEndpoint {
                entity,
                which,
                start_cursor,
                dragged,
            };
        }
    }
}

pub fn handle_edit_click_end(
    mode: Res<Mode>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    mut drag: ResMut<EditDragState>,
) {
    if *mode != Mode::Edit || !buttons.just_released(MouseButton::Left) {
        return;
    }
    let pressed = match *drag {
        EditDragState::Pressed { entity, dragged, .. }
        | EditDragState::CableBody { entity, dragged, .. }
        | EditDragState::CableEndpoint { entity, dragged, .. } => Some((entity, dragged)),
        EditDragState::Idle => None,
    };
    *drag = EditDragState::Idle;

    let Some((entity, dragged)) = pressed else {
        return;
    };
    if dragged {
        // The move was already applied live in `handle_edit_drag`.
        return;
    }

    // Cables no longer reference `Pin` entities (connectivity is spatial),
    // so despawning here — recursively taking a component's pins/label
    // children with it, same as always — needs no extra cascade.
    commands.entity(entity).despawn();
}
