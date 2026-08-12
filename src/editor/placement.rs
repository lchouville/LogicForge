use bevy::prelude::*;

use crate::simulation::components::GateKind;

use super::resources::{ArmedTool, PickCycleState, ToolKind};
use super::spawn::{spawn_and_or_gate, spawn_lamp, spawn_not_gate, spawn_switch};

pub fn handle_tool_arming(keys: Res<ButtonInput<KeyCode>>, mut armed: ResMut<ArmedTool>) {
    if keys.just_pressed(KeyCode::Digit1) {
        armed.0 = Some(ToolKind::Gate(GateKind::And));
    } else if keys.just_pressed(KeyCode::Digit2) {
        armed.0 = Some(ToolKind::Gate(GateKind::Or));
    } else if keys.just_pressed(KeyCode::Digit3) {
        armed.0 = Some(ToolKind::Gate(GateKind::Not));
    } else if keys.just_pressed(KeyCode::Digit4) {
        armed.0 = Some(ToolKind::Switch);
    } else if keys.just_pressed(KeyCode::Digit5) {
        armed.0 = Some(ToolKind::Lamp);
    } else if keys.just_pressed(KeyCode::Digit6) {
        armed.0 = Some(ToolKind::Cable);
    } else if keys.just_pressed(KeyCode::Digit0) || keys.just_pressed(KeyCode::Escape) {
        armed.0 = None;
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

pub fn place_tool(commands: &mut Commands, tool: ToolKind, cell: IVec2, z: f32) {
    match tool {
        ToolKind::Gate(GateKind::And) => {
            spawn_and_or_gate(commands, cell, GateKind::And, z);
        }
        ToolKind::Gate(GateKind::Or) => {
            spawn_and_or_gate(commands, cell, GateKind::Or, z);
        }
        ToolKind::Gate(GateKind::Not) => {
            spawn_not_gate(commands, cell, z);
        }
        ToolKind::Switch => {
            spawn_switch(commands, cell, z);
        }
        ToolKind::Lamp => {
            spawn_lamp(commands, cell, z);
        }
        ToolKind::Cable => {
            // Cables are placed via press+drag (see `handle_left_click_start`
            // / `handle_left_click_end`), not a single-click `place_tool`
            // call, since they need a start *and* end cell.
        }
    }
}
