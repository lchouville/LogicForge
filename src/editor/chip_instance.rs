use bevy::prelude::*;

use crate::constants::{
    COLOR_BUTTON_BORDER, COLOR_BUTTON_NORMAL, COLOR_CHIP_SOCKET_LIT, COLOR_NEUTRAL,
    COLOR_STRUCTURE_LAMP, GRID_CELL_SIZE, LABEL_FONT_SIZE, PREVIEW_Z, UI_FONT_PATH,
};
use crate::grid::{cell_to_world, world_to_cell};
use crate::rendering::appearance::PendingAppearance;
use crate::simulation::components::{Cable, GridPosition, Pin, PinRole, SignalValue};
use crate::simulation::logic::{LogicState, read_logic};

use super::chip_structure::{StructureBlockKind, StructurePinLabel};
use super::hud::StandardEditorUi;
use super::preview::{PlacementPreview, PlacementPreviewTint};
use super::project::{ProjectId, ProjectLibrary, SavedEntity, spawn_saved_entity};
use super::resources::{ArmedTool, ChipInstanceSlotAllocator, ToolKind};
use super::spawn::{facing_quat, placeholder_sprite};

/// A placed copy of another project's structure ("puce"), frozen at the
/// moment of placement — see `ProjectLibrary::chip_blueprint`/`chip_interior`.
/// Its Pin/Lamp blocks are real, cable-connectable net-resolution
/// participants (see `spawn_chip_instance`), and — since `interior` below —
/// its own interior circuit genuinely simulates too. `blocks`/`interior`
/// carry just enough to redraw and respawn without ever looking `source`
/// back up in `ProjectLibrary` again, so a later rename/deletion of the
/// source project can't desync or break an already-placed instance.
#[derive(Component, Clone)]
pub struct ChipInstance {
    pub source: ProjectId,
    pub display_name: String,
    pub body_color: Color,
    pub blocks: Vec<(IVec2, StructureBlockKind, String)>,
    /// The source project's own interior circuit (gates/switches/lamps/
    /// `PinHeader`s/cables — and possibly further nested `ChipInstance`s of
    /// its own), frozen at the moment *this* chip was placed — see
    /// `ProjectLibrary::chip_interior`. Empty means "nothing to simulate",
    /// which `spawn_chip_instance` treats as a plain no-op (the exact
    /// pre-nested-simulation behavior). A nested `SavedEntity::Chip` in here
    /// always carries its own already-resolved `interior` in turn (it was
    /// frozen when *it* was placed), so a single `.clone()` of this field is
    /// enough to carry arbitrarily deep nesting — no recursive resolution
    /// needed at spawn time, and no cycle is possible: placing project A
    /// inside project B requires A to be visited-and-non-active (hence
    /// already frozen at that moment), so any apparent A-in-B-in-A nesting
    /// is really just snapshots taken at different times, never a live loop.
    pub interior: Vec<SavedEntity>,
}

/// Marks a `Cable` as an internal, invisible link between a placed chip's
/// exterior Pin/Lamp socket and the matching-label `PinHeader` in its
/// private interior circuit (see `spawn_chip_instance`) — not a wire the
/// player ever drew. Carries no `Transform`/`Children`/`Sprite`, so
/// `rendering::cable::rebuild_cable_segments`/`sync_cable_sprite` (both
/// require `&Children`) already skip it for free; this marker exists to
/// also defensively exclude it from cable selection/drag/deletion queries
/// (`wiring::find_cable_at`, `edit_mode.rs`), so the player can never select
/// or delete a bridge that might span thousands of grid cells to a chip's
/// private simulation space.
#[derive(Component)]
pub(crate) struct ChipBridgeCable;

/// The absolute grid cell a local offset (already the same units
/// `cell_to_world` produces, e.g. `cell_to_world(local_cell)` or a fixed
/// pin-lead offset like `spawn_pin_header`'s own `Vec2::new(GRID_CELL_SIZE,
/// 0.0)`) resolves to once `root_world`/`rotation` are applied — the exact
/// math Bevy's own transform propagation would produce for a one-level-deep
/// child, just computed synchronously so a bridge `Cable`'s `start`/`end`
/// can be baked in at spawn time instead of waiting a frame for
/// `GlobalTransform` to catch up.
fn absolute_cell(root_world: Vec2, rotation: u8, local_offset: Vec2) -> IVec2 {
    let transform =
        Transform::from_translation(root_world.extend(0.0)).with_rotation(facing_quat(rotation));
    world_to_cell(
        transform
            .transform_point(local_offset.extend(0.0))
            .truncate(),
    )
}

/// Marks a placed `ChipInstance`'s Pin/Lamp socket for
/// `sync_chip_instance_socket_color`'s dedicated lit/unlit tint (plain
/// white when HIGH, `off_color` otherwise) — deliberately overrides the
/// generic `rendering::sync::sync_pin_colors` HIGH/LOW/NEUTRAL diagnostic
/// palette for these sockets specifically (registered `.after()` it in
/// `plugin.rs`, so this tint always wins the same frame), so a placed
/// chip's connection points read as a simple lit/unlit indicator rather
/// than the raw signal-sign palette used everywhere else.
#[derive(Component)]
pub(crate) struct ChipInstanceSocket {
    off_color: Color,
}

/// Places a frozen copy of another project's structure as a component in
/// the current one. `blocks` positions are local-structure-space cells
/// (the same numbering the structure editor itself uses, unrelated to
/// `STRUCTURE_SPACE_OFFSET` which only parks that editor's own live
/// entities far away in world-space) — used directly as child offsets here,
/// so whichever cell the chip's designer treated as their own local
/// `(0, 0)` becomes this instance's placement anchor.
pub fn spawn_chip_instance(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cell: IVec2,
    rotation: u8,
    instance: ChipInstance,
    z: f32,
    slots: &mut ChipInstanceSlotAllocator,
) -> Entity {
    let world = cell_to_world(cell);
    let blocks = instance.blocks.clone();
    let body_color = instance.body_color;
    let interior = instance.interior.clone();
    // Exterior socket absolute cells, keyed by label — computed once here
    // (before `instance` moves into `commands.spawn` below) so the
    // bridge-wiring pass after the interior spawn doesn't need to redo this
    // per candidate. A block with no label, or a Corps, never appears here
    // and so can never gain a bridge — exactly today's behavior.
    let exterior_sockets: Vec<(String, IVec2)> = blocks
        .iter()
        .filter(|(_, kind, label)| {
            matches!(kind, StructureBlockKind::Pin | StructureBlockKind::Lamp) && !label.is_empty()
        })
        .map(|(local_cell, _, label)| {
            (
                label.clone(),
                absolute_cell(world, rotation, cell_to_world(*local_cell)),
            )
        })
        .collect();
    let mut root = commands.spawn((
        instance,
        GridPosition(cell),
        Transform::from_translation(world.extend(z)).with_rotation(facing_quat(rotation)),
        Visibility::default(),
    ));
    root.with_children(|parent| {
        for (local_cell, kind, label) in &blocks {
            let offset = Transform::from_translation(cell_to_world(*local_cell).extend(0.0));
            // Same per-kind appearance as `chip_structure::spawn_structure_block`
            // (Corps tinted by `body_color` with `structure_body.json`, Pin
            // with the shared `pin.json` socket art, Lampe a flat
            // `COLOR_STRUCTURE_LAMP` square, no async asset) but without
            // `StructureCell` — a placed instance's blocks are a read-only
            // snapshot, not editable structure-editor entities. Duplicated
            // rather than reusing `spawn_structure_block` directly since
            // that function always spawns its own root entity (never a
            // child) and unconditionally tags `StructureCell`, which this
            // copy must not carry.
            //
            // Pin/Lamp additionally get `Pin`/`SignalValue` — real
            // net-resolution participants, cable-connectable exactly like
            // `pin_header::spawn_pin_header`'s own child (same
            // `PinRole::Input` passive-sink default, direction deferred to
            // the future nested-simulation chantier) — plus the frozen
            // `StructurePinLabel` for that same future pairing, and
            // `ChipInstanceSocket` for the lit/unlit tint. Corps stays
            // purely visual: nothing to connect, nothing to light up.
            match kind {
                StructureBlockKind::Body => {
                    parent.spawn((
                        offset,
                        placeholder_sprite(body_color, 1.0, 1.0),
                        PendingAppearance(asset_server.load("appearances/structure_body.json")),
                    ));
                }
                StructureBlockKind::Pin => {
                    parent.spawn((
                        offset,
                        placeholder_sprite(COLOR_NEUTRAL, 1.0, 1.0),
                        PendingAppearance(asset_server.load("appearances/pin.json")),
                        Pin {
                            role: PinRole::Input,
                            index: 0,
                        },
                        SignalValue::default(),
                        StructurePinLabel(label.clone()),
                        ChipInstanceSocket {
                            off_color: COLOR_NEUTRAL,
                        },
                    ));
                }
                StructureBlockKind::Lamp => {
                    parent.spawn((
                        offset,
                        placeholder_sprite(COLOR_STRUCTURE_LAMP, 1.0, 1.0),
                        Pin {
                            role: PinRole::Input,
                            index: 0,
                        },
                        SignalValue::default(),
                        StructurePinLabel(label.clone()),
                        ChipInstanceSocket {
                            off_color: COLOR_STRUCTURE_LAMP,
                        },
                    ));
                }
            }
        }

        // Same "leg" visual (bridging a Pin to an orthogonally-adjacent
        // Corps/Lampe block) as `chip_structure::sync_structure_pin_legs`,
        // but computed once here rather than as a reactive system: a placed
        // instance's blocks are frozen at placement time and never change,
        // so there's nothing to recompute later — unlike the structure
        // editor's own blocks, which the player can add/move/delete live.
        let attach_cells: Vec<IVec2> = blocks
            .iter()
            .filter(|(_, kind, _)| {
                matches!(kind, StructureBlockKind::Body | StructureBlockKind::Lamp)
            })
            .map(|(cell, _, _)| *cell)
            .collect();
        const NEIGHBOR_OFFSETS: [IVec2; 4] = [
            IVec2::new(1, 0),
            IVec2::new(-1, 0),
            IVec2::new(0, 1),
            IVec2::new(0, -1),
        ];
        for (cell, kind, _) in &blocks {
            if !matches!(kind, StructureBlockKind::Pin) {
                continue;
            }
            for leg_offset in NEIGHBOR_OFFSETS {
                let neighbor = *cell + leg_offset;
                if !attach_cells.contains(&neighbor) {
                    continue;
                }
                let mid_local = (cell_to_world(*cell) + cell_to_world(neighbor)) / 2.0;
                // `pin_lead.json` is authored as a horizontal bar — rotate a
                // quarter turn for a vertically-adjacent pin, same reasoning
                // as `sync_structure_pin_legs`.
                let rotation = if leg_offset.x != 0 {
                    Quat::IDENTITY
                } else {
                    Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)
                };
                parent.spawn((
                    Sprite::default(),
                    PendingAppearance(asset_server.load("appearances/pin_lead.json")),
                    Transform::from_translation(mid_local.extend(-0.05)).with_rotation(rotation),
                ));
            }
        }
    });
    let root_entity = root.id();

    // Spawn a private, translated copy of the source project's own interior
    // circuit — see `ChipInstanceSlotAllocator`'s doc comment for why a
    // fresh, never-reused offset here is enough to guarantee it never
    // collides with any other placed chip's own private circuit, at any
    // nesting depth. Empty `interior` (no source circuit, or a chip placed
    // before this chantier's data existed) is a no-op — same shell-only
    // behavior as before nested simulation existed.
    if !interior.is_empty() {
        let interior_offset = slots.allocate();
        let mut interior_z = 0.0_f32;
        for saved in &interior {
            spawn_saved_entity(
                commands,
                asset_server,
                saved,
                interior_offset,
                &mut interior_z,
                slots,
            );
        }
        // Bridge every exterior Pin/Lamp socket to the interior `PinHeader`
        // sharing its label (the same match `chip_structure`/`pin_header`'s
        // "Lié" indicator already computes for display) with one bare,
        // invisible `Cable` — see `ChipBridgeCable`'s doc comment for why
        // this needs no `Transform`/`Sprite`/`Children` despite potentially
        // spanning thousands of cells to the interior's private space.
        for saved in &interior {
            let SavedEntity::Pin {
                cell: pin_cell,
                rotation: pin_rotation,
                label,
            } = saved
            else {
                continue;
            };
            if label.is_empty() {
                continue;
            }
            let Some((_, exterior_cell)) = exterior_sockets
                .iter()
                .find(|(candidate, _)| candidate == label)
            else {
                continue;
            };
            let interior_world = cell_to_world(*pin_cell + interior_offset);
            // `spawn_pin_header`'s own single pin child sits one full cell
            // to the right of its own root (`Vec2::new(GRID_CELL_SIZE,
            // 0.0)` in `src/editor/spawn.rs`) — reproduced here since a
            // `PinHeader`'s electrical cell is that pin's cell, not its
            // root's.
            let interior_cell = absolute_cell(
                interior_world,
                *pin_rotation,
                Vec2::new(GRID_CELL_SIZE, 0.0),
            );
            commands.spawn((
                Cable {
                    start: *exterior_cell,
                    end: interior_cell,
                },
                SignalValue::default(),
                ChipBridgeCable,
            ));
        }
    }

    root_entity
}

/// Tints every placed `ChipInstance`'s Pin/Lamp socket white when it carries
/// a HIGH signal, its own base color otherwise — see `ChipInstanceSocket`'s
/// doc comment for why this exists separately from (and must run after) the
/// generic `rendering::sync::sync_pin_colors`.
pub fn sync_chip_instance_socket_color(
    mut sockets: Query<(&ChipInstanceSocket, &SignalValue, &mut Sprite)>,
) {
    for (socket, signal, mut sprite) in &mut sockets {
        sprite.color = if read_logic(signal.0) == LogicState::High {
            COLOR_CHIP_SOCKET_LIT
        } else {
            socket.off_color
        };
    }
}

/// Same footprint as `spawn_chip_instance`, dimmed by
/// `tint_placement_preview`, minus the `ChipInstance`/`GridPosition`
/// components a preview ghost must never carry — mirrors
/// `spawn::spawn_placement_preview`'s own reasoning.
pub fn spawn_chip_instance_preview(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cell: IVec2,
    rotation: u8,
    body_color: Color,
    blocks: &[(IVec2, StructureBlockKind, String)],
) {
    let world = cell_to_world(cell);
    commands
        .spawn((
            PlacementPreview,
            Transform::from_translation(world.extend(PREVIEW_Z))
                .with_rotation(facing_quat(rotation)),
            Visibility::default(),
        ))
        .with_children(|parent| {
            // `tint_placement_preview` overwrites every `PlacementPreviewTint`
            // sprite's color to plain dimmed white every frame regardless of
            // what's set here (see its doc comment) — same reasoning
            // `spawn_placement_preview`'s own branches rely on, so the real
            // (non-dimmed) colors are passed here unchanged.
            for (local_cell, kind, _label) in blocks {
                let offset = Transform::from_translation(cell_to_world(*local_cell).extend(0.0));
                match kind {
                    StructureBlockKind::Body => {
                        parent.spawn((
                            PlacementPreviewTint,
                            offset,
                            placeholder_sprite(body_color, 1.0, 1.0),
                            PendingAppearance(asset_server.load("appearances/structure_body.json")),
                        ));
                    }
                    StructureBlockKind::Pin => {
                        parent.spawn((
                            PlacementPreviewTint,
                            offset,
                            placeholder_sprite(COLOR_NEUTRAL, 1.0, 1.0),
                            PendingAppearance(asset_server.load("appearances/pin.json")),
                        ));
                    }
                    StructureBlockKind::Lamp => {
                        parent.spawn((
                            PlacementPreviewTint,
                            offset,
                            placeholder_sprite(COLOR_STRUCTURE_LAMP, 1.0, 1.0),
                        ));
                    }
                }
            }
        });
}

/// Whether the chip picker list (below the toggle button) is expanded —
/// same on/off pattern as `sidebar::SidebarOpen`.
#[derive(Resource, Default)]
pub struct ChipPickerOpen(pub bool);

#[derive(Component)]
pub(crate) struct ChipPickerToggleButton;

#[derive(Component)]
pub(crate) struct ChipPickerBody;

#[derive(Component)]
pub(crate) struct ChipPickerRowsContainer;

#[derive(Component, Clone, Copy)]
pub(crate) struct ChipPickerRow(pub ProjectId);

fn chip_picker_text_font(font: Handle<Font>) -> impl Bundle {
    (
        TextFont {
            font: font.into(),
            font_size: LABEL_FONT_SIZE.into(),
            ..default()
        },
        TextColor(Color::WHITE),
    )
}

/// Same collapse/expand shape as `sidebar::spawn_sidebar` (always-visible
/// toggle handle + a fully collapsible body), positioned bottom-left above
/// the component toolbar so it doesn't overlap `hud::spawn_toolbar`.
/// `StandardEditorUi`-tagged like the rest of the interior editor's own HUD:
/// placing a chip only makes sense there, and its click-handling systems are
/// already gated to that view — untagged, the button would sit visible but
/// dead while the structure editor is showing.
pub fn spawn_chip_picker(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT_PATH);
    commands
        .spawn((
            StandardEditorUi,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(56.0),
                left: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Button,
                ChipPickerToggleButton,
                Node {
                    height: Val::Px(28.0),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(COLOR_BUTTON_NORMAL),
                BorderColor::all(COLOR_BUTTON_BORDER),
                children![(Text::new("Puce"), chip_picker_text_font(font.clone()))],
            ));
            parent.spawn((
                ChipPickerBody,
                Node {
                    display: Display::None,
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(6.0)),
                    row_gap: Val::Px(4.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(COLOR_BUTTON_NORMAL),
                BorderColor::all(COLOR_BUTTON_BORDER),
                children![(
                    ChipPickerRowsContainer,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                )],
            ));
        });
}

pub fn handle_chip_picker_toggle_click(
    mut open: ResMut<ChipPickerOpen>,
    toggle_button: Query<&Interaction, (Changed<Interaction>, With<ChipPickerToggleButton>)>,
) {
    if toggle_button
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        open.0 = !open.0;
    }
}

pub fn sync_chip_picker_collapse(
    open: Res<ChipPickerOpen>,
    mut body: Query<&mut Node, With<ChipPickerBody>>,
) {
    if !open.is_changed() {
        return;
    }
    let Ok(mut node) = body.single_mut() else {
        return;
    };
    node.display = if open.0 { Display::Flex } else { Display::None };
}

/// Rebuilds the picker's row list from every project *other* than the
/// active one that actually has a placeable structure — same
/// despawn-and-respawn-from-scratch precedent as
/// `sidebar::sync_project_rows`, and the same `chip_blueprint(id).is_some()`
/// check `handle_left_click_start` uses at placement time, so a project
/// never shows here only to silently no-op when picked.
pub fn sync_chip_picker_rows(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    library: Res<ProjectLibrary>,
    container: Query<(Entity, Option<&Children>), With<ChipPickerRowsContainer>>,
) {
    if !library.is_changed() {
        return;
    }
    let Ok((container_entity, children)) = container.single() else {
        return;
    };
    if let Some(children) = children {
        for &child in children {
            commands.entity(child).despawn();
        }
    }

    let font = asset_server.load(UI_FONT_PATH);
    commands.entity(container_entity).with_children(|parent| {
        for entry in &library.entries {
            if entry.id == library.active {
                continue;
            }
            // Reuses `chip_blueprint`'s own name resolution (structure_label
            // if the player set one, else the project's own name) so the
            // picker shows exactly what a placed instance's Nom will read.
            let Some((display_name, _, _)) = library.chip_blueprint(entry.id) else {
                continue;
            };
            parent.spawn((
                Button,
                ChipPickerRow(entry.id),
                Node {
                    padding: UiRect::all(Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(COLOR_BUTTON_NORMAL),
                BorderColor::all(COLOR_BUTTON_BORDER),
                children![(Text::new(display_name), chip_picker_text_font(font.clone()))],
            ));
        }
    });
}

/// Arms `ToolKind::Chip(id)` and closes the picker — placement itself then
/// goes through the already-generic `handle_left_click_start`/`place_tool`
/// path, same as any other tool.
pub fn handle_chip_picker_row_click(
    mut armed: ResMut<ArmedTool>,
    mut open: ResMut<ChipPickerOpen>,
    rows: Query<(&Interaction, &ChipPickerRow), Changed<Interaction>>,
) {
    let Some((_, row)) = rows
        .iter()
        .find(|(interaction, _)| **interaction == Interaction::Pressed)
    else {
        return;
    };
    armed.0 = Some(ToolKind::Chip(row.0));
    open.0 = false;
}
