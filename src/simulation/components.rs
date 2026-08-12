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
pub struct Wire {
    pub from: Entity,
    pub to: Entity,
}

/// Scratch buffer used to hand values from a read-only propagation/evaluation
/// pass to the following write pass, since a system can't hold both `&SignalValue`
/// and `&mut SignalValue` queries at once when they may alias the same entities.
#[derive(Resource, Default)]
pub struct SignalWriteBuffer(pub Vec<(Entity, f32)>);
