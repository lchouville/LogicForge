use bevy::prelude::*;

use crate::simulation::components::GateKind;

use super::pin_header::PinHeaderLabelFocus;
use super::project::ProjectId;
use super::wiring::CableEnd;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Gate(GateKind),
    Switch,
    Lamp,
    Cable,
    Pin,
    /// A placed copy of another project's structure — see
    /// `chip_instance::ChipInstance`. Carries which project to copy from,
    /// resolved to a frozen blueprint only at actual placement time.
    Chip(ProjectId),
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
/// hold-drag a component to move it, plain-click it to select it (Delete/
/// Backspace removes the current `Selected` entity).
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Interaction,
    #[default]
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

/// The entity (placed component or cable) selected by a plain, non-dragged
/// click in Edit mode — highlighted by `render_selection_highlight` and
/// removed by `handle_delete_selected` on Delete/Backspace. Cleared on
/// clicking empty space, switching mode, or deleting the selection itself.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub struct Selected(pub Option<Entity>);

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

/// Bundles every piece of transient editor-interaction state into one
/// `SystemParam` — `reset_transient_editor_state`/`project::switch_to_project`
/// both need write access to all of it at once, and spelling out six separate
/// `ResMut` parameters on top of everything else a system like
/// `sidebar::handle_project_selection` already needs pushes it past Bevy's
/// 16-parameter limit for a plain function system.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TransientEditorState<'w> {
    pub armed: ResMut<'w, ArmedTool>,
    pub interaction: ResMut<'w, InteractionState>,
    pub drag: ResMut<'w, EditDragState>,
    pub cycle: ResMut<'w, PickCycleState>,
    pub selected: ResMut<'w, Selected>,
    pub spawn_order: ResMut<'w, SpawnOrderCounter>,
    /// A focused `PinHeader` label field is stale the instant its target
    /// entity is despawned by a project switch — same reasoning as
    /// `selected` above.
    pub pin_header_label_focus: ResMut<'w, PinHeaderLabelFocus>,
}
