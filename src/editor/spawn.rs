use bevy::prelude::*;

use crate::constants::{
    COLOR_GATE, COLOR_LAMP_OFF, COLOR_NEUTRAL, COLOR_SWITCH, GRID_CELL_SIZE, LABEL_FONT_SIZE,
    PREVIEW_ALPHA, PREVIEW_Z,
};
use crate::grid::cell_to_world;
use crate::rendering::appearance::PendingAppearance;
use crate::simulation::components::{
    GateKind, GridPosition, Lamp, Pin, PinRole, SignalValue, Switch,
};

use super::preview::{PlacementPreview, PlacementPreviewTint};
use super::resources::ToolKind;

// Pins sit exactly one grid step away from the body's anchor row, i.e. on a
// neighbouring grid node, so cables always run node-to-node instead of a
// fractional offset.
const PIN_X_OFFSET: f32 = GRID_CELL_SIZE;
const PIN_Y_OFFSET: f32 = GRID_CELL_SIZE;

/// Turns a pre-placement facing (0-3 quarter-turns, see `PendingRotation`)
/// into the root `Transform`'s rotation. Every pin/leg/label offset below is
/// authored for facing 0 as a plain local `Vec2` on a child of the root, so
/// rotating just the root here is enough to correctly carry the whole
/// layout with it through Bevy's normal transform propagation — including,
/// conveniently, net resolution's `world_to_cell` reads of each pin's
/// `GlobalTransform`, since every offset here is an exact multiple of
/// `GRID_CELL_SIZE` and a quarter-turn of an exact grid offset is still an
/// exact grid offset. Negated so increasing `rotation` turns clockwise on
/// screen (Bevy's positive Z-rotation is counter-clockwise).
pub(crate) fn facing_quat(rotation: u8) -> Quat {
    Quat::from_rotation_z(-(rotation as f32) * std::f32::consts::FRAC_PI_2)
}
// A 2-input gate's footprint is 3 nodes wide x 2 nodes tall: the two input
// rows ARE the two grid nodes (anchor row + one row up), not the anchor row
// +/- a full row each, which would make the pins' own cells overhang the
// body by half a row and inflate the footprint to 3 tall. The body sprite
// (2 blocks tall) is centered on the midpoint between those two rows, i.e.
// nudged up by half a cell from the anchor.
pub(crate) const GATE_BODY_ROW_OFFSET: f32 = GRID_CELL_SIZE / 2.0;

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

/// A pin socket: the `Pin` entity renders the plug (`pin.json`), tinted per
/// signal state by `sync_pin_colors`. No longer carries its own `node.json`
/// bracket child — the background grid (`background_grid.rs`) already tiles
/// that same art under every cell, so a per-pin copy was just a redundant
/// extra frame stacked on top of it.
fn pin(asset_server: &AssetServer, role: PinRole, index: u8, offset: Vec2) -> impl Bundle {
    (
        Pin { role, index },
        SignalValue::default(),
        placeholder_sprite(COLOR_NEUTRAL, 1.0, 1.0),
        PendingAppearance(asset_server.load("appearances/pin.json")),
        Transform::from_translation(offset.extend(1.0)),
    )
}

/// A visual-only stand-in for `pin()`, used by the placement preview: same
/// `pin.json` art and offset as a real pin, but none of the logic
/// components (`Pin`, `SignalValue`) — a preview ghost must never become a
/// real net-resolution node.
fn pin_ghost(asset_server: &AssetServer, offset: Vec2) -> impl Bundle {
    (
        placeholder_sprite(COLOR_NEUTRAL, 1.0, 1.0),
        PendingAppearance(asset_server.load("appearances/pin.json")),
        Transform::from_translation(offset.extend(1.0)),
    )
}

/// A short metal "leg" bridging the visual gap between a pin's own drawn
/// circle and the body's drawn edge — the two grid cells already touch, but
/// each piece of art has its own margin inside its cell, so without this a
/// component pin reads as floating rather than attached (like a chip's
/// legs). Only used for a component's own pins, positioned by the caller
/// halfway between that pin and the body — never for cable endpoints, which
/// have no fixed body to point back at. Static (not signal-tinted), same as
/// `node.json`.
fn leg(asset_server: &AssetServer, offset: Vec2) -> impl Bundle {
    (
        Sprite::default(),
        PendingAppearance(asset_server.load("appearances/pin_lead.json")),
        // Behind the body (whose local/root z is 0.0), not in front of it:
        // the half of the leg that overlaps the body's own drawn footprint
        // gets hidden underneath it, so the leg reads as emerging from
        // under the body's edge rather than sitting beside it.
        Transform::from_translation(offset.extend(-0.1)),
    )
}

fn label(text: &str, offset: Vec2, color: Color) -> impl Bundle {
    (
        Text2d::new(text),
        TextFont {
            font_size: LABEL_FONT_SIZE.into(),
            ..default()
        },
        TextColor(color),
        Transform::from_translation(offset.extend(2.0)),
    )
}

/// A component body: the pixel-art appearance placed at a fixed local
/// offset from the entity's `GridPosition` anchor (rather than directly on
/// the root entity), so multi-row components (see `GATE_BODY_ROW_OFFSET`)
/// can center their body between rows while the root's own `Transform`
/// stays exactly `cell_to_world(cell)` — which drag-to-move (`edit_mode.rs`)
/// and click hit-testing both rely on staying anchor-cell-accurate.
fn body(
    asset_server: &AssetServer,
    path: &'static str,
    color: Color,
    blocks_wide: f32,
    blocks_tall: f32,
    offset: Vec2,
) -> impl Bundle {
    (
        placeholder_sprite(color, blocks_wide, blocks_tall),
        PendingAppearance(asset_server.load(path)),
        Transform::from_translation(offset.extend(0.0)),
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
    rotation: u8,
    z: f32,
) -> Entity {
    let world = cell_to_world(cell);
    let body_offset = Vec2::new(0.0, GATE_BODY_ROW_OFFSET);
    commands
        .spawn((
            kind,
            GridPosition(cell),
            Transform::from_translation(world.extend(z)).with_rotation(facing_quat(rotation)),
            Visibility::default(),
            children![
                // 2 blocks tall, centered between the anchor row and the row
                // above it — see `GATE_BODY_ROW_OFFSET`.
                body(
                    asset_server,
                    gate_appearance_path(kind),
                    COLOR_GATE,
                    1.0,
                    2.0,
                    body_offset
                ),
                // Anchor row (row 0): the second input and the output.
                pin(
                    asset_server,
                    PinRole::Input,
                    1,
                    Vec2::new(-PIN_X_OFFSET, 0.0)
                ),
                leg(asset_server, Vec2::new(-PIN_X_OFFSET / 2.0, 0.0)),
                pin(
                    asset_server,
                    PinRole::Output,
                    0,
                    Vec2::new(PIN_X_OFFSET, 0.0)
                ),
                leg(asset_server, Vec2::new(PIN_X_OFFSET / 2.0, 0.0)),
                // Row above the anchor (row +1): the first input.
                pin(
                    asset_server,
                    PinRole::Input,
                    0,
                    Vec2::new(-PIN_X_OFFSET, PIN_Y_OFFSET)
                ),
                leg(asset_server, Vec2::new(-PIN_X_OFFSET / 2.0, PIN_Y_OFFSET)),
                label(gate_label(kind), body_offset, Color::WHITE),
            ],
        ))
        .id()
}

pub fn spawn_not_gate(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cell: IVec2,
    rotation: u8,
    z: f32,
) -> Entity {
    let world = cell_to_world(cell);
    commands
        .spawn((
            GateKind::Not,
            GridPosition(cell),
            placeholder_sprite(COLOR_GATE, 1.0, 1.0),
            PendingAppearance(asset_server.load(gate_appearance_path(GateKind::Not))),
            Transform::from_translation(world.extend(z)).with_rotation(facing_quat(rotation)),
            children![
                pin(
                    asset_server,
                    PinRole::Input,
                    0,
                    Vec2::new(-PIN_X_OFFSET, 0.0)
                ),
                leg(asset_server, Vec2::new(-PIN_X_OFFSET / 2.0, 0.0)),
                pin(
                    asset_server,
                    PinRole::Output,
                    0,
                    Vec2::new(PIN_X_OFFSET, 0.0)
                ),
                leg(asset_server, Vec2::new(PIN_X_OFFSET / 2.0, 0.0)),
                label(gate_label(GateKind::Not), Vec2::ZERO, Color::WHITE),
            ],
        ))
        .id()
}

pub fn spawn_switch(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cell: IVec2,
    rotation: u8,
    z: f32,
) -> Entity {
    let world = cell_to_world(cell);
    commands
        .spawn((
            Switch { on: false },
            GridPosition(cell),
            placeholder_sprite(COLOR_SWITCH, 1.0, 1.0),
            PendingAppearance(asset_server.load("appearances/switch.json")),
            Transform::from_translation(world.extend(z)).with_rotation(facing_quat(rotation)),
            children![
                pin(
                    asset_server,
                    PinRole::Output,
                    0,
                    Vec2::new(PIN_X_OFFSET, 0.0)
                ),
                leg(asset_server, Vec2::new(PIN_X_OFFSET / 2.0, 0.0)),
                label("SW", Vec2::ZERO, Color::WHITE),
            ],
        ))
        .id()
}

pub fn spawn_lamp(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cell: IVec2,
    rotation: u8,
    z: f32,
) -> Entity {
    let world = cell_to_world(cell);
    commands
        .spawn((
            Lamp,
            GridPosition(cell),
            placeholder_sprite(COLOR_LAMP_OFF, 1.0, 1.0),
            PendingAppearance(asset_server.load("appearances/lamp.json")),
            Transform::from_translation(world.extend(z)).with_rotation(facing_quat(rotation)),
            children![
                pin(
                    asset_server,
                    PinRole::Input,
                    0,
                    Vec2::new(-PIN_X_OFFSET, 0.0)
                ),
                leg(asset_server, Vec2::new(-PIN_X_OFFSET / 2.0, 0.0)),
                label("LMP", Vec2::ZERO, Color::WHITE),
            ],
        ))
        .id()
}

/// Ghosts the given tool's full body + legs + pins + label at `cell`,
/// dimmed by `tint_placement_preview` (and the label's own pre-dimmed
/// color, since `Text2d` has no `Sprite` for that system to tint). Mirrors
/// each `spawn_*` function's layout above exactly, minus every logic
/// component (`GateKind`, `GridPosition`, `Pin`, `SignalValue`, `Switch`,
/// `Lamp`) — a preview must be purely visual, never a real entity the
/// simulation or net resolution would pick up. No case for `ToolKind::Cable`
/// — it gets its own live preview once the player starts dragging (see
/// `render_cable_drag_preview`), and has no single fixed footprint to ghost
/// before that.
pub fn spawn_placement_preview(
    commands: &mut Commands,
    asset_server: &AssetServer,
    tool: ToolKind,
    cell: IVec2,
    rotation: u8,
) {
    let world = cell_to_world(cell);
    let label_color = Color::WHITE.with_alpha(PREVIEW_ALPHA);

    // Built with `.with_children()` (imperative spawn-per-child) rather than
    // the `children![...]` macro the real `spawn_*` functions above use —
    // no functional difference, just what this was rewritten to while
    // chasing an unrelated bug (see `PREVIEW_Z`'s doc comment).
    let mut root = commands.spawn((
        PlacementPreview,
        Transform::from_translation(world.extend(PREVIEW_Z)).with_rotation(facing_quat(rotation)),
        Visibility::default(),
    ));

    match tool {
        ToolKind::Gate(kind @ (GateKind::And | GateKind::Or)) => {
            let body_offset = Vec2::new(0.0, GATE_BODY_ROW_OFFSET);
            root.with_children(|parent| {
                parent.spawn((
                    PlacementPreviewTint,
                    body(
                        asset_server,
                        gate_appearance_path(kind),
                        COLOR_GATE,
                        1.0,
                        2.0,
                        body_offset,
                    ),
                ));
                parent.spawn((
                    PlacementPreviewTint,
                    pin_ghost(asset_server, Vec2::new(-PIN_X_OFFSET, 0.0)),
                ));
                parent.spawn((
                    PlacementPreviewTint,
                    leg(asset_server, Vec2::new(-PIN_X_OFFSET / 2.0, 0.0)),
                ));
                parent.spawn((
                    PlacementPreviewTint,
                    pin_ghost(asset_server, Vec2::new(PIN_X_OFFSET, 0.0)),
                ));
                parent.spawn((
                    PlacementPreviewTint,
                    leg(asset_server, Vec2::new(PIN_X_OFFSET / 2.0, 0.0)),
                ));
                parent.spawn((
                    PlacementPreviewTint,
                    pin_ghost(asset_server, Vec2::new(-PIN_X_OFFSET, PIN_Y_OFFSET)),
                ));
                parent.spawn((
                    PlacementPreviewTint,
                    leg(asset_server, Vec2::new(-PIN_X_OFFSET / 2.0, PIN_Y_OFFSET)),
                ));
                parent.spawn(label(gate_label(kind), body_offset, label_color));
            });
        }
        ToolKind::Gate(GateKind::Not) => {
            root.with_children(|parent| {
                parent.spawn((
                    PlacementPreviewTint,
                    body(
                        asset_server,
                        gate_appearance_path(GateKind::Not),
                        COLOR_GATE,
                        1.0,
                        1.0,
                        Vec2::ZERO,
                    ),
                ));
                parent.spawn((
                    PlacementPreviewTint,
                    pin_ghost(asset_server, Vec2::new(-PIN_X_OFFSET, 0.0)),
                ));
                parent.spawn((
                    PlacementPreviewTint,
                    leg(asset_server, Vec2::new(-PIN_X_OFFSET / 2.0, 0.0)),
                ));
                parent.spawn((
                    PlacementPreviewTint,
                    pin_ghost(asset_server, Vec2::new(PIN_X_OFFSET, 0.0)),
                ));
                parent.spawn((
                    PlacementPreviewTint,
                    leg(asset_server, Vec2::new(PIN_X_OFFSET / 2.0, 0.0)),
                ));
                parent.spawn(label(gate_label(GateKind::Not), Vec2::ZERO, label_color));
            });
        }
        ToolKind::Switch => {
            root.with_children(|parent| {
                parent.spawn((
                    PlacementPreviewTint,
                    body(
                        asset_server,
                        "appearances/switch.json",
                        COLOR_SWITCH,
                        1.0,
                        1.0,
                        Vec2::ZERO,
                    ),
                ));
                parent.spawn((
                    PlacementPreviewTint,
                    pin_ghost(asset_server, Vec2::new(PIN_X_OFFSET, 0.0)),
                ));
                parent.spawn((
                    PlacementPreviewTint,
                    leg(asset_server, Vec2::new(PIN_X_OFFSET / 2.0, 0.0)),
                ));
                parent.spawn(label("SW", Vec2::ZERO, label_color));
            });
        }
        ToolKind::Lamp => {
            root.with_children(|parent| {
                parent.spawn((
                    PlacementPreviewTint,
                    body(
                        asset_server,
                        "appearances/lamp.json",
                        COLOR_LAMP_OFF,
                        1.0,
                        1.0,
                        Vec2::ZERO,
                    ),
                ));
                parent.spawn((
                    PlacementPreviewTint,
                    pin_ghost(asset_server, Vec2::new(-PIN_X_OFFSET, 0.0)),
                ));
                parent.spawn((
                    PlacementPreviewTint,
                    leg(asset_server, Vec2::new(-PIN_X_OFFSET / 2.0, 0.0)),
                ));
                parent.spawn(label("LMP", Vec2::ZERO, label_color));
            });
        }
        ToolKind::Cable => {}
    }
}
