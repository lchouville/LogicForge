use bevy::prelude::*;

use crate::simulation::components::GateKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Gate(GateKind),
    Switch,
    Lamp,
}

/// The component the player has armed via a number key. `None` means
/// "interact mode": clicks act on existing entities (toggle / wire / edit)
/// instead of placing something new.
#[derive(Resource, Default)]
pub struct ArmedTool(pub Option<ToolKind>);

#[derive(Resource, Default, Clone, Copy)]
pub enum InteractionState {
    #[default]
    Idle,
    Dragging {
        from_pin: Entity,
    },
}

/// Which of the two mutually exclusive click modes is active (toggled by a
/// single key). Interaction: toggle switches, drag wires between pins. Edit:
/// hold-drag a component to move it, plain-click it to delete it.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Interaction,
    Edit,
}

#[derive(Resource, Default, Clone, Copy)]
pub enum EditDragState {
    #[default]
    Idle,
    Pressed {
        entity: Entity,
        start_cursor: Vec2,
        dragged: bool,
    },
}
