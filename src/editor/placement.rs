use bevy::prelude::*;

use crate::simulation::components::{GateKind, GridPosition};

use super::resources::{ArmedTool, ToolKind};
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
    } else if keys.just_pressed(KeyCode::Digit0) || keys.just_pressed(KeyCode::Escape) {
        armed.0 = None;
    }
}

pub fn is_cell_occupied(cell: IVec2, positions: &Query<&GridPosition>) -> bool {
    positions.iter().any(|position| position.0 == cell)
}

pub fn place_tool(commands: &mut Commands, tool: ToolKind, cell: IVec2) {
    match tool {
        ToolKind::Gate(GateKind::And) => {
            spawn_and_or_gate(commands, cell, GateKind::And);
        }
        ToolKind::Gate(GateKind::Or) => {
            spawn_and_or_gate(commands, cell, GateKind::Or);
        }
        ToolKind::Gate(GateKind::Not) => {
            spawn_not_gate(commands, cell);
        }
        ToolKind::Switch => {
            spawn_switch(commands, cell);
        }
        ToolKind::Lamp => {
            spawn_lamp(commands, cell);
        }
    }
}
