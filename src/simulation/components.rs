use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPosition(pub IVec2);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateKind {
    And,
    Or,
    Not,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Switch {
    pub on: bool,
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Lamp;

/// A native, cable-connectable "Pin" component placed in the interior
/// circuit — the future link target for a chip's exterior structure Pin/Lamp
/// (see `editor::chip_structure::StructurePinLabel`): matching labels on
/// each side mark them as linked. Electrically a passive sink for now (its
/// one child `Pin` is `PinRole::Input`, same as `Lamp`) — no live signal
/// actually crosses into/out of a placed chip yet, since cross-project chip
/// instancing doesn't exist; direction (this pin as a chip input vs. output)
/// is deferred to that future chantier, where it'll actually be observable.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PinHeader;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinRole {
    Input,
    Output,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pin {
    pub role: PinRole,
    pub index: u8,
}

#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct SignalValue(pub f32);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cable {
    pub start: IVec2,
    pub end: IVec2,
}

/// Scratch buffer used to hand values from a read-only propagation/evaluation
/// pass to the following write pass, since a system can't hold both `&SignalValue`
/// and `&mut SignalValue` queries at once when they may alias the same entities.
#[derive(Resource, Default)]
pub struct SignalWriteBuffer(pub Vec<(Entity, f32)>);
