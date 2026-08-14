use bevy::prelude::*;

use crate::constants::{
    COLOR_GATE, COLOR_LAMP_OFF, COLOR_NEUTRAL, COLOR_SWITCH, GRID_CELL_SIZE, LABEL_FONT_SIZE,
};
use crate::grid::cell_to_world;
use crate::rendering::appearance::PendingAppearance;
use crate::simulation::components::{
    GateKind, GridPosition, Lamp, Pin, PinRole, SignalValue, Switch,
};

// Pins sit exactly one grid step away from the body, i.e. on the neighbouring
// grid node, so cables always run node-to-node instead of a fractional offset.
const PIN_X_OFFSET: f32 = GRID_CELL_SIZE;
const PIN_Y_OFFSET: f32 = GRID_CELL_SIZE;

/// A flat-color placeholder shown at roughly the final footprint while a
/// body's real pixel-art appearance streams in asynchronously (see
/// `PendingAppearance` / `apply_loaded_appearances`).
fn placeholder_sprite(color: Color, blocks_wide: f32, blocks_tall: f32) -> Sprite {
    Sprite {
        color,
        custom_size: Some(Vec2::new(
            GRID_CELL_SIZE * blocks_wide,
            GRID_CELL_SIZE * blocks_tall,
        )),
        ..default()
    }
}

/// A pin socket: the `Pin` entity itself renders the plug (`pin.json`),
/// tinted per signal state by `sync_pin_colors` exactly as the old flat
/// square was; a child carries the static bracket/socket art (`node.json`)
/// behind it, never tinted, so a pin reads as "plugged into a grid node"
/// per the reference art instead of a bare colored square.
fn pin(asset_server: &AssetServer, role: PinRole, index: u8, offset: Vec2) -> impl Bundle {
    (
        Pin { role, index },
        SignalValue::default(),
        placeholder_sprite(COLOR_NEUTRAL, 1.0, 1.0),
        PendingAppearance(asset_server.load("appearances/pin.json")),
        Transform::from_translation(offset.extend(1.0)),
        children![(
            Sprite::default(),
            PendingAppearance(asset_server.load("appearances/node.json")),
            Transform::from_xyz(0.0, 0.0, -0.5),
        )],
    )
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

fn gate_appearance_path(kind: GateKind) -> &'static str {
    match kind {
        GateKind::And => "appearances/and_gate.json",
        GateKind::Or => "appearances/or_gate.json",
        GateKind::Not => "appearances/not_gate.json",
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
    commands
        .spawn((
            kind,
            GridPosition(cell),
            // 2 blocks tall: the body spans the full gap between the two
            // input rows either side of center, per the reference art.
            placeholder_sprite(COLOR_GATE, 1.0, 2.0),
            PendingAppearance(asset_server.load(gate_appearance_path(kind))),
            Transform::from_translation(world.extend(z)),
            children![
                pin(
                    asset_server,
                    PinRole::Input,
                    0,
                    Vec2::new(-PIN_X_OFFSET, PIN_Y_OFFSET)
                ),
                pin(
                    asset_server,
                    PinRole::Input,
                    1,
                    Vec2::new(-PIN_X_OFFSET, -PIN_Y_OFFSET)
                ),
                pin(
                    asset_server,
                    PinRole::Output,
                    0,
                    Vec2::new(PIN_X_OFFSET, 0.0)
                ),
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
            placeholder_sprite(COLOR_GATE, 1.0, 1.0),
            PendingAppearance(asset_server.load(gate_appearance_path(GateKind::Not))),
            Transform::from_translation(world.extend(z)),
            children![
                pin(
                    asset_server,
                    PinRole::Input,
                    0,
                    Vec2::new(-PIN_X_OFFSET, 0.0)
                ),
                pin(
                    asset_server,
                    PinRole::Output,
                    0,
                    Vec2::new(PIN_X_OFFSET, 0.0)
                ),
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
            placeholder_sprite(COLOR_SWITCH, 1.0, 1.0),
            PendingAppearance(asset_server.load("appearances/switch.json")),
            Transform::from_translation(world.extend(z)),
            children![
                pin(
                    asset_server,
                    PinRole::Output,
                    0,
                    Vec2::new(PIN_X_OFFSET, 0.0)
                ),
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
            placeholder_sprite(COLOR_LAMP_OFF, 1.0, 1.0),
            PendingAppearance(asset_server.load("appearances/lamp.json")),
            Transform::from_translation(world.extend(z)),
            children![
                pin(
                    asset_server,
                    PinRole::Input,
                    0,
                    Vec2::new(-PIN_X_OFFSET, 0.0)
                ),
                label("LMP"),
            ],
        ))
        .id()
}

