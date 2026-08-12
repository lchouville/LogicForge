use bevy::prelude::*;

use crate::constants::COLOR_NEUTRAL;
use crate::grid::world_to_cell;
use crate::simulation::components::{GridPosition, Pin, PinRole, SignalValue, Switch, Wire};

use super::cursor::cursor_world_position;
use super::hud::PointerOverUi;
use super::placement::{is_cell_occupied, place_tool};
use super::resources::{ArmedTool, InteractionState, Mode};
use super::wiring::{find_pin_at, is_pin_wired, is_valid_wire_target};

#[allow(clippy::too_many_arguments)]
pub fn handle_left_click_start(
    buttons: Res<ButtonInput<MouseButton>>,
    mode: Res<Mode>,
    pointer_over_ui: Res<PointerOverUi>,
    window: Single<&Window>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    mut commands: Commands,
    mut armed: ResMut<ArmedTool>,
    mut interaction: ResMut<InteractionState>,
    positions: Query<&GridPosition>,
    pins: Query<(Entity, &Pin, &GlobalTransform)>,
    mut switches: Query<(&GridPosition, &mut Switch, &Children)>,
    mut signals: Query<&mut SignalValue>,
) {
    if pointer_over_ui.0 || !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let (camera, camera_transform) = *camera_query;
    let Some(world_pos) = cursor_world_position(&window, camera, camera_transform) else {
        return;
    };

    if let Some(tool) = armed.0 {
        let cell = world_to_cell(world_pos);
        if !is_cell_occupied(cell, &positions) {
            place_tool(&mut commands, tool, cell);
        }
        // Matches the design doc's default "select + click" behaviour: the tool
        // disarms after one placement. Holding a modifier for repeated placement
        // is a later addition, not required for the MVP kernel.
        armed.0 = None;
        return;
    }

    // Toggling/wiring only applies in Interaction mode; Edit mode's own click
    // handlers own move/delete instead.
    if *mode != Mode::Interaction {
        return;
    }

    if let Some((pin_entity, PinRole::Output)) = find_pin_at(world_pos, &pins) {
        *interaction = InteractionState::Dragging {
            from_pin: pin_entity,
        };
        return;
    }

    let cell = world_to_cell(world_pos);
    for (position, mut switch, children) in &mut switches {
        if position.0 != cell {
            continue;
        }
        switch.on = !switch.on;
        let value = if switch.on { 1.0 } else { 0.0 };
        for child in children.iter() {
            if let Ok(mut signal) = signals.get_mut(child) {
                signal.0 = value;
            }
        }
        break;
    }
}

pub fn render_wire_drag_preview(
    interaction: Res<InteractionState>,
    window: Single<&Window>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    pins: Query<&GlobalTransform, With<Pin>>,
    mut gizmos: Gizmos,
) {
    let InteractionState::Dragging { from_pin } = *interaction else {
        return;
    };
    let (camera, camera_transform) = *camera_query;
    let Some(cursor_world) = cursor_world_position(&window, camera, camera_transform) else {
        return;
    };
    let Ok(from_transform) = pins.get(from_pin) else {
        return;
    };
    gizmos.line_2d(
        from_transform.translation().truncate(),
        cursor_world,
        COLOR_NEUTRAL,
    );
}

pub fn handle_left_click_end(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    mut commands: Commands,
    mut interaction: ResMut<InteractionState>,
    pins: Query<(Entity, &Pin, &GlobalTransform)>,
    wires: Query<&Wire>,
) {
    if !buttons.just_released(MouseButton::Left) {
        return;
    }
    let InteractionState::Dragging { from_pin } = *interaction else {
        return;
    };
    *interaction = InteractionState::Idle;

    let (camera, camera_transform) = *camera_query;
    let Some(world_pos) = cursor_world_position(&window, camera, camera_transform) else {
        return;
    };
    let Some((target_pin, target_role)) = find_pin_at(world_pos, &pins) else {
        return;
    };

    let already_wired = is_pin_wired(target_pin, &wires);
    if is_valid_wire_target(PinRole::Output, target_role, already_wired) {
        commands.spawn(Wire {
            from: from_pin,
            to: target_pin,
        });
    }
}
