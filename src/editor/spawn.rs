use bevy::prelude::*;

use crate::constants::{
    COLOR_GATE, COLOR_LAMP_OFF, COLOR_NEUTRAL, COLOR_SWITCH, GRID_CELL_SIZE, LABEL_FONT_SIZE,
};
use crate::grid::cell_to_world;
use crate::rendering::appearance::PendingAppearance;
use crate::simulation::components::{
    Cable, GateKind, GridPosition, Lamp, Pin, PinRole, SignalValue, Switch,
};

const BODY_SIZE: f32 = GRID_CELL_SIZE * 0.9;
const PIN_SIZE: f32 = 8.0;
// Pins sit exactly one grid step away from the body, i.e. on the neighbouring
// grid node, so wires always run node-to-node instead of a fractional offset.
const PIN_X_OFFSET: f32 = GRID_CELL_SIZE;
const PIN_Y_OFFSET: f32 = GRID_CELL_SIZE;

fn pin(role: PinRole, index: u8, offset: Vec2) -> impl Bundle {
    (
        Pin { role, index },
        SignalValue::default(),
        Sprite {
            color: COLOR_NEUTRAL,
            custom_size: Some(Vec2::splat(PIN_SIZE)),
            ..default()
        },
        Transform::from_translation(offset.extend(1.0)),
    )
}

fn body_sprite(color: Color) -> Sprite {
    Sprite {
        color,
        custom_size: Some(Vec2::splat(BODY_SIZE)),
        ..default()
    }
}

fn label(text: &str) -> impl Bundle {
    (
        Text2d::new(text),
        TextFont {
            font_size: LABEL_FONT_SIZE.into(),
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
    )
}

fn gate_label(kind: GateKind) -> &'static str {
    match kind {
        GateKind::And => "AND",
        GateKind::Or => "OR",
        GateKind::Not => "NOT",
    }
}

pub fn spawn_and_or_gate(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cell: IVec2,
    kind: GateKind,
    z: f32,
) -> Entity {
    let world = cell_to_world(cell);
    let appearance_path = match kind {
        GateKind::And => "appearances/and_gate.json",
        GateKind::Or => "appearances/or_gate.json",
        GateKind::Not => "appearances/not_gate.json",
    };
    commands
        .spawn((
            kind,
            GridPosition(cell),
            body_sprite(COLOR_GATE),
            PendingAppearance(asset_server.load(appearance_path)),
            Transform::from_translation(world.extend(z)),
            children![
                pin(PinRole::Input, 0, Vec2::new(-PIN_X_OFFSET, PIN_Y_OFFSET)),
                pin(PinRole::Input, 1, Vec2::new(-PIN_X_OFFSET, -PIN_Y_OFFSET)),
                pin(PinRole::Output, 0, Vec2::new(PIN_X_OFFSET, 0.0)),
                label(gate_label(kind)),
            ],
        ))
        .id()
}

pub fn spawn_not_gate(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cell: IVec2,
    z: f32,
) -> Entity {
    let world = cell_to_world(cell);
    commands
        .spawn((
            GateKind::Not,
            GridPosition(cell),
            body_sprite(COLOR_GATE),
            PendingAppearance(asset_server.load("appearances/not_gate.json")),
            Transform::from_translation(world.extend(z)),
            children![
                pin(PinRole::Input, 0, Vec2::new(-PIN_X_OFFSET, 0.0)),
                pin(PinRole::Output, 0, Vec2::new(PIN_X_OFFSET, 0.0)),
                label(gate_label(GateKind::Not)),
            ],
        ))
        .id()
}

pub fn spawn_switch(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cell: IVec2,
    z: f32,
) -> Entity {
    let world = cell_to_world(cell);
    commands
        .spawn((
            Switch { on: false },
            GridPosition(cell),
            body_sprite(COLOR_SWITCH),
            PendingAppearance(asset_server.load("appearances/switch.json")),
            Transform::from_translation(world.extend(z)),
            children![
                pin(PinRole::Output, 0, Vec2::new(PIN_X_OFFSET, 0.0)),
                label("SW"),
            ],
        ))
        .id()
}

pub fn spawn_lamp(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cell: IVec2,
    z: f32,
) -> Entity {
    let world = cell_to_world(cell);
    commands
        .spawn((
            Lamp,
            GridPosition(cell),
            body_sprite(COLOR_LAMP_OFF),
            PendingAppearance(asset_server.load("appearances/lamp.json")),
            Transform::from_translation(world.extend(z)),
            children![
                pin(PinRole::Input, 0, Vec2::new(-PIN_X_OFFSET, 0.0)),
                label("LMP"),
            ],
        ))
        .id()
}

pub fn spawn_cable(commands: &mut Commands, start: IVec2, end: IVec2) -> Entity {
    commands
        .spawn((Cable { start, end }, SignalValue::default()))
        .id()
}
