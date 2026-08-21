use bevy::prelude::*;

use crate::simulation::components::GateKind;

use super::resources::{ArmedTool, PendingRotation, PickCycleState, ToolKind};
use super::spawn::{spawn_and_or_gate, spawn_lamp, spawn_not_gate, spawn_pin_header, spawn_switch};

pub fn handle_tool_arming(
    keys: Res<ButtonInput<KeyCode>>,
    mut armed: ResMut<ArmedTool>,
    mut rotation: ResMut<PendingRotation>,
) {
    let newly_armed = if keys.just_pressed(KeyCode::Digit1) {
        Some(Some(ToolKind::Gate(GateKind::And)))
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(Some(ToolKind::Gate(GateKind::Or)))
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(Some(ToolKind::Gate(GateKind::Not)))
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(Some(ToolKind::Switch))
    } else if keys.just_pressed(KeyCode::Digit5) {
        Some(Some(ToolKind::Lamp))
    } else if keys.just_pressed(KeyCode::Digit6) {
        Some(Some(ToolKind::Cable))
    } else if keys.just_pressed(KeyCode::Digit7) {
        Some(Some(ToolKind::Pin))
    } else if keys.just_pressed(KeyCode::Digit0) || keys.just_pressed(KeyCode::Escape) {
        Some(None)
    } else {
        None
    };

    if let Some(tool) = newly_armed {
        armed.0 = tool;
        rotation.0 = 0;
    }
}

/// Rotates the armed tool's placement preview (and the component it'll spawn)
/// in 90-degree steps before it's placed: `R` and the right arrow both
/// advance clockwise, left arrow backs up. No-op with nothing armed, or with
/// the cable tool armed (a cable's orientation comes from where it's
/// dragged, not a pre-placement facing).
pub fn handle_rotation_input(
    keys: Res<ButtonInput<KeyCode>>,
    armed: Res<ArmedTool>,
    mut rotation: ResMut<PendingRotation>,
) {
    let Some(tool) = armed.0 else {
        return;
    };
    if tool == ToolKind::Cable {
        return;
    }

    if keys.just_pressed(KeyCode::KeyR) || keys.just_pressed(KeyCode::ArrowRight) {
        rotation.0 = (rotation.0 + 1) % 4;
    } else if keys.just_pressed(KeyCode::ArrowLeft) {
        rotation.0 = (rotation.0 + 3) % 4;
    }
}

/// Picks which entity a click at `cell` should act on when several occupy
/// the same cell (components may now overlap). Repeated clicks at the same
/// cell cycle through the candidates (Figma-style) instead of always landing
/// on the same one; a click at a different cell restarts the cycle.
/// `candidates` is sorted before indexing so the choice is stable across
/// frames regardless of query iteration order.
pub fn pick_entity_at_cell(
    cell: IVec2,
    mut candidates: Vec<Entity>,
    cycle: &mut PickCycleState,
) -> Option<Entity> {
    candidates.sort();
    if candidates.is_empty() {
        return None;
    }
    if cycle.last_cell != Some(cell) {
        cycle.last_cell = Some(cell);
        cycle.index = 0;
    } else {
        cycle.index = (cycle.index + 1) % candidates.len();
    }
    candidates.get(cycle.index).copied()
}

pub fn place_tool(
    commands: &mut Commands,
    asset_server: &AssetServer,
    tool: ToolKind,
    cell: IVec2,
    rotation: u8,
    z: f32,
) {
    match tool {
        ToolKind::Gate(GateKind::And) => {
            spawn_and_or_gate(commands, asset_server, cell, GateKind::And, rotation, z);
        }
        ToolKind::Gate(GateKind::Or) => {
            spawn_and_or_gate(commands, asset_server, cell, GateKind::Or, rotation, z);
        }
        ToolKind::Gate(GateKind::Not) => {
            spawn_not_gate(commands, asset_server, cell, rotation, z);
        }
        ToolKind::Switch => {
            spawn_switch(commands, asset_server, cell, rotation, z);
        }
        ToolKind::Lamp => {
            spawn_lamp(commands, asset_server, cell, rotation, z);
        }
        ToolKind::Pin => {
            spawn_pin_header(commands, asset_server, cell, rotation, "", z);
        }
        ToolKind::Cable => {
            // Cables are placed via press+drag (see `handle_left_click_start`
            // / `handle_left_click_end`), not a single-click `place_tool`
            // call, since they need a start *and* end cell (and have no
            // pre-placement facing to rotate — see `handle_rotation_input`).
        }
    }
}
