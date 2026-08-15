use bevy::prelude::*;

use crate::simulation::components::GateKind;

use super::wiring::CableEnd;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Gate(GateKind),
    Switch,
    Lamp,
    Cable,
}

/// The component the player has armed via a number key. `None` means
/// "interact mode": clicks act on existing entities (toggle / wire / edit)
/// instead of placing something new.
#[derive(Resource, Default)]
pub struct ArmedTool(pub Option<ToolKind>);

/// The armed tool's pending orientation, in quarter-turns (0-3) applied
/// clockwise before placement — `R` or the right arrow key advances it,
/// left arrow backs it up. Reset to 0 whenever a tool is (re)armed, so it
/// never silently carries a stale rotation into an unrelated placement; not
/// meaningful for `ToolKind::Cable`, whose orientation comes from the drag
/// itself.
#[derive(Resource, Default)]
pub struct PendingRotation(pub u8);

#[derive(Resource, Default, Clone, Copy)]
pub enum InteractionState {
    #[default]
    Idle,
    /// A cable is being traced: the press-cell is fixed as the start, the
    /// end follows the cursor until release.
    PlacingCable { start_cell: IVec2 },
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
    CableBody {
        entity: Entity,
        start_cursor: Vec2,
        orig_start: IVec2,
        orig_end: IVec2,
        dragged: bool,
    },
    CableEndpoint {
        entity: Entity,
        which: CableEnd,
        start_cursor: Vec2,
        dragged: bool,
    },
}

/// Tracks which candidate a repeated click at the same grid cell should
/// select next, so overlapping components/switches can be cycled through
/// (Figma-style) instead of only ever reaching whichever one a query happens
/// to visit first.
#[derive(Resource, Default)]
pub struct PickCycleState {
    pub last_cell: Option<IVec2>,
    pub index: usize,
}

/// Monotonically increasing z-offset handed out per placement so overlapping
/// component bodies (now allowed) have a deterministic, later-on-top draw
/// order instead of an unstable one from same-z sprite batching.
#[derive(Resource, Default)]
pub struct SpawnOrderCounter(pub f32);
