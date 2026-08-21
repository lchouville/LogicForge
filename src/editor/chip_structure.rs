use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use crate::constants::{
    COLOR_BUTTON_ARMED, COLOR_BUTTON_BORDER, COLOR_BUTTON_NORMAL, COLOR_HOVER, COLOR_NEUTRAL,
    COLOR_SELECTION, COLOR_STRUCTURE_LAMP, EDIT_DRAG_THRESHOLD, GRID_CELL_SIZE, LABEL_FONT_SIZE,
    SELECTION_OUTLINE_MARGIN, SPAWN_Z_STEP, STRUCTURE_COLOR_PALETTE, STRUCTURE_SPACE_OFFSET,
    UI_FONT_PATH,
};
use crate::grid::{cell_to_world, world_to_cell};
use crate::rendering::appearance::PendingAppearance;

use super::camera_control::{CameraPanState, PanSource};
use super::hud::PointerOverUi;
use super::pointer::PointerState;
use super::project::ProjectView;
use super::resources::SpawnOrderCounter;
use super::spawn::placeholder_sprite;

/// A structure block's position in the chip's own local coordinate space —
/// before `STRUCTURE_SPACE_OFFSET` is added to get its world position. Kept
/// separate from `simulation::components::GridPosition` on purpose: the two
/// never mix, so `project::switch_to_project`'s interior/structure
/// snapshot+despawn queries can each filter on their own marker without a
/// risk of catching the other space's entities.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub struct StructureCell(pub IVec2);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum StructureBlockKind {
    /// Structural block — placing several (adjacent or not) is how the
    /// player builds the chip's exterior silhouette. Tinted by
    /// `ActiveStructureColor`.
    Body,
    /// Connection point — placed but inert for now (see roadmap notes: the
    /// input/output link to the interior circuit is a follow-up chantier).
    Pin,
    /// Visual indicator block (future: reflects a linked interior signal).
    Lamp,
}

/// Fixed, non-editable explanation of what a structure block type is for —
/// same role as `inspector::selected_component_info`'s descriptions for
/// native gates, so every element in the editor (native or structure) reads
/// as documented by the game rather than by the player. Unlike a chip's own
/// name/description (which *will* be player-authored once roadmap item 5's
/// cross-project reuse system exists), a block's *type* meaning is fixed —
/// only an actual placed chip's identity is ever custom text.
fn structure_block_description(kind: StructureBlockKind) -> &'static str {
    match kind {
        StructureBlockKind::Body => {
            "Corps : bloc de structure qui forme la silhouette extérieure de la puce."
        }
        StructureBlockKind::Pin => "Pin : sert à faire une entrée ou une sortie de puce.",
        StructureBlockKind::Lamp => {
            "Lampe : bloc indicateur qui reflète l'état d'un signal du circuit intérieur."
        }
    }
}

#[derive(Resource, Default)]
pub struct ArmedStructureTool(pub Option<StructureBlockKind>);

/// The structure block selected by a plain, non-dragged click when no tool
/// is armed — mirrors `resources::Selected`, namespaced to this separate
/// entity space so a stale reference here can never point at (or get
/// confused with) an interior circuit entity. Cleared on clicking empty
/// space, switching project/view (see `project::ViewSwitchState`), or
/// deleting the selection itself.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub struct SelectedStructureBlock(pub Option<Entity>);

/// Mirrors `resources::EditDragState`, minus the cable-specific variants —
/// a structure block is always a single 1x1 point, so there's only one
/// shape of "currently pressed" to track.
#[derive(Resource, Default, Clone, Copy)]
pub enum StructureDragState {
    #[default]
    Idle,
    Pressed {
        entity: Entity,
        start_cursor: Vec2,
        dragged: bool,
    },
}

/// The Corps (body) tint currently chosen from `STRUCTURE_COLOR_PALETTE` for
/// the active project's structure.
#[derive(Resource, Clone, Copy)]
pub struct ActiveStructureColor(pub Color);

impl Default for ActiveStructureColor {
    fn default() -> Self {
        ActiveStructureColor(STRUCTURE_COLOR_PALETTE[0])
    }
}

/// The chip's name, as typed into the structure toolbar's text field (see
/// `StructureLabelField`) — shown both there and on the world-space label
/// above the structure (`StructureNameLabel`, `sync_structure_name_label`).
/// Same per-project role as `ActiveStructureColor`, just for text.
#[derive(Resource, Default, Clone)]
pub struct ActiveStructureLabel(pub String);

/// Whether the toolbar's name field is currently capturing keystrokes — see
/// `handle_structure_label_field_click` / `handle_structure_label_typing`.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub struct StructureLabelFocus(pub bool);

#[derive(Component)]
pub(crate) struct StructureToolbar;

#[derive(Component, Clone, Copy)]
pub(crate) struct StructureToolButton(StructureBlockKind);

#[derive(Component, Clone, Copy)]
pub(crate) struct StructureColorButton(Color);

/// Touch/click equivalent of `Delete`/`Backspace` — mirrors `hud::DeleteButton`.
#[derive(Component)]
pub(crate) struct StructureDeleteButton;

/// The clickable name field itself — clicking it toggles `StructureLabelFocus`.
#[derive(Component)]
pub(crate) struct StructureLabelField;

/// The `Text` child of `StructureLabelField` that
/// `sync_structure_label_field_text` keeps in sync with `ActiveStructureLabel`.
#[derive(Component)]
pub(crate) struct StructureLabelText;

/// Shown in the name field in place of an empty `ActiveStructureLabel`.
const STRUCTURE_LABEL_PLACEHOLDER: &str = "Nom de la puce";

/// A Pin or Lamp block's own label — future link key between this external
/// connection point and a precise point in the interior circuit (not wired
/// up yet, this is just the data + editing UI): the player will match this
/// label against an interior Pin's own label to say "this is the same
/// signal". Attached to `StructureBlockKind::Pin` and `StructureBlockKind::Lamp`
/// entities only — a Corps has nothing to connect, so it never carries one;
/// empty means "not labeled yet".
#[derive(Component, Clone, Default)]
pub struct StructurePinLabel(pub String);

/// Whether the floating per-pin label panel (`StructurePinLabelPanel`) is
/// currently capturing keystrokes — mirrors `StructureLabelFocus`, kept
/// separate since the two fields (chip name vs. a specific pin's label)
/// can never be focused at the same time but are otherwise independent.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub struct StructurePinLabelFocus(pub bool);

/// Spawns one 1x1 structure block. `Pin` reuses the exact same visual as an
/// interior pin (`spawn::pin`'s `pin.json` appearance, same
/// `COLOR_NEUTRAL` placeholder while it loads) rather than a flat placeholder
/// square, so it reads as the same "connection point" the player already
/// recognizes from the interior circuit editor. `Body`/`Lamp` have no
/// equivalent art yet, so they stay flat-colored.
#[allow(clippy::too_many_arguments)]
pub fn spawn_structure_block(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cell: IVec2,
    kind: StructureBlockKind,
    body_color: Color,
    initial_label: &str,
    z: f32,
) -> Entity {
    let world = cell_to_world(cell) + STRUCTURE_SPACE_OFFSET;
    let mut entity = commands.spawn((
        StructureCell(cell),
        kind,
        Transform::from_translation(world.extend(z)),
    ));
    match kind {
        StructureBlockKind::Body => {
            entity.insert((
                placeholder_sprite(body_color, 1.0, 1.0),
                PendingAppearance(asset_server.load("appearances/structure_body.json")),
            ));
        }
        StructureBlockKind::Pin => {
            entity.insert((
                placeholder_sprite(COLOR_NEUTRAL, 1.0, 1.0),
                PendingAppearance(asset_server.load("appearances/pin.json")),
                StructurePinLabel(initial_label.to_string()),
            ));
        }
        StructureBlockKind::Lamp => {
            entity.insert((
                placeholder_sprite(COLOR_STRUCTURE_LAMP, 1.0, 1.0),
                StructurePinLabel(initial_label.to_string()),
            ));
        }
    }
    entity.id()
}

/// Click-to-place, or click-to-select-and-start-a-drag, for the chip
/// structure editor — the structure-space equivalent of
/// `interaction::handle_left_click_start` + `edit_mode::handle_edit_click_start`
/// combined into one rule, since (unlike the interior editor) there's no
/// separate Interaction/Edit mode here: an armed tool means "placing",
/// nothing armed means "select/move/delete" — same split the interior
/// editor gets from `ArmedTool`+`Mode` together, just collapsed onto the
/// one resource this simpler editor has.
#[allow(clippy::too_many_arguments)]
pub fn handle_structure_click(
    pointer: Res<PointerState>,
    pointer_over_ui: Res<PointerOverUi>,
    armed: Res<ArmedStructureTool>,
    color: Res<ActiveStructureColor>,
    asset_server: Res<AssetServer>,
    mut spawn_order: ResMut<SpawnOrderCounter>,
    mut selected: ResMut<SelectedStructureBlock>,
    mut drag: ResMut<StructureDragState>,
    mut commands: Commands,
    blocks: Query<(Entity, &StructureCell)>,
) {
    if !pointer.just_pressed || pointer_over_ui.0 {
        return;
    }
    let Some(world_pos) = pointer.world_pos else {
        return;
    };
    let cell = world_to_cell(world_pos - STRUCTURE_SPACE_OFFSET);
    let existing = blocks
        .iter()
        .find(|(_, structure_cell)| structure_cell.0 == cell)
        .map(|(entity, _)| entity);

    if let Some(kind) = armed.0 {
        // An armed tool only places on empty cells — it never selects or
        // deletes an existing block, matching the interior editor's own
        // placement tools (which likewise never act on whatever's already
        // under the cursor).
        if existing.is_none() {
            let z = spawn_order.0;
            spawn_order.0 += SPAWN_Z_STEP;
            spawn_structure_block(&mut commands, &asset_server, cell, kind, color.0, "", z);
        }
        return;
    }

    match existing {
        Some(entity) => {
            selected.0 = Some(entity);
            *drag = StructureDragState::Pressed {
                entity,
                start_cursor: world_pos,
                dragged: false,
            };
        }
        // Clicked empty space with nothing armed: drop the current
        // selection, same as the interior editor's Edit mode.
        None => selected.0 = None,
    }
}

/// Live-updates a dragged block's cell/position — mirrors
/// `edit_mode::handle_edit_drag`'s `Pressed` branch (the only one that
/// applies here, since structure blocks have no cable-body/cable-endpoint
/// equivalent).
pub fn handle_structure_drag(
    pointer: Res<PointerState>,
    mut drag: ResMut<StructureDragState>,
    mut positioned: Query<(&mut StructureCell, &mut Transform)>,
) {
    if !pointer.pressed {
        return;
    }
    let Some(world_pos) = pointer.world_pos else {
        return;
    };
    let StructureDragState::Pressed {
        entity,
        start_cursor,
        mut dragged,
    } = *drag
    else {
        return;
    };

    if !dragged && start_cursor.distance(world_pos) > EDIT_DRAG_THRESHOLD {
        dragged = true;
    }
    if dragged && let Ok((mut cell, mut transform)) = positioned.get_mut(entity) {
        let target_cell = world_to_cell(world_pos - STRUCTURE_SPACE_OFFSET);
        if cell.0 != target_cell {
            cell.0 = target_cell;
            transform.translation = (cell_to_world(target_cell) + STRUCTURE_SPACE_OFFSET)
                .extend(transform.translation.z);
        }
    }
    *drag = StructureDragState::Pressed {
        entity,
        start_cursor,
        dragged,
    };
}

/// Ends a press-drag started by `handle_structure_click` — mirrors
/// `edit_mode::handle_edit_click_end`.
pub fn handle_structure_click_end(
    pointer: Res<PointerState>,
    mut drag: ResMut<StructureDragState>,
) {
    if !pointer.just_released {
        return;
    }
    *drag = StructureDragState::Idle;
}

/// Structure-space equivalent of `camera_control::handle_camera_pan` — same
/// `CameraPanState` state machine and drag-from-empty-space convention, but
/// decides whether to start a pan using `ArmedStructureTool` and
/// `StructureCell` occupancy instead of the interior editor's own
/// `ArmedTool`/`GridPosition`/`Cable`, none of which mean anything in
/// structure space. Kept as its own system (mirroring `handle_structure_click`
/// vs `handle_left_click_start`/`handle_edit_click_start`) rather than
/// teaching `handle_camera_pan` about both spaces — the interior editor's own
/// pan was already debugged once for a judder bug (see
/// `notes/claude/2026-08-18.md`), so it's left untouched rather than risking
/// a regression there.
#[allow(clippy::too_many_arguments)]
pub fn handle_structure_camera_pan(
    mouse: Res<ButtonInput<MouseButton>>,
    pointer: Res<PointerState>,
    touches: Res<Touches>,
    armed: Res<ArmedStructureTool>,
    pointer_over_ui: Res<PointerOverUi>,
    blocks: Query<&StructureCell>,
    mut pan: ResMut<CameraPanState>,
    camera_query: Single<(&Camera, &GlobalTransform, &mut Transform), With<Camera2d>>,
) {
    let (camera, camera_transform, mut transform) = camera_query.into_inner();

    if touches.iter().count() >= 2
        && !matches!(
            *pan,
            CameraPanState::Panning {
                source: PanSource::Middle,
                ..
            }
        )
    {
        *pan = CameraPanState::Idle;
    }

    if mouse.just_pressed(MouseButton::Middle)
        && let Some(screen_pos) = pointer.screen_pos
    {
        *pan = CameraPanState::Panning {
            last_screen_pos: screen_pos,
            source: PanSource::Middle,
        };
    } else if pointer.just_pressed
        && armed.0.is_none()
        && !pointer_over_ui.0
        && let Some(world_pos) = pointer.world_pos
    {
        let cell = world_to_cell(world_pos - STRUCTURE_SPACE_OFFSET);
        let occupied = blocks.iter().any(|structure_cell| structure_cell.0 == cell);
        if !occupied {
            *pan = CameraPanState::Pressed {
                start_cursor: world_pos,
            };
        }
    }

    match *pan {
        CameraPanState::Idle => {}
        CameraPanState::Pressed { start_cursor } => {
            if !pointer.pressed {
                *pan = CameraPanState::Idle;
            } else if let Some(world_pos) = pointer.world_pos
                && let Some(screen_pos) = pointer.screen_pos
                && start_cursor.distance(world_pos) > EDIT_DRAG_THRESHOLD
            {
                *pan = CameraPanState::Panning {
                    last_screen_pos: screen_pos,
                    source: PanSource::Pointer,
                };
            }
        }
        CameraPanState::Panning {
            last_screen_pos,
            source,
        } => {
            let still_pressed = match source {
                PanSource::Middle => mouse.pressed(MouseButton::Middle),
                PanSource::Pointer => pointer.pressed,
            };
            if !still_pressed {
                *pan = CameraPanState::Idle;
            } else if let Some(current_world) = pointer.world_pos
                && let Some(current_screen) = pointer.screen_pos
                && let Ok(last_world_reprojected) =
                    camera.viewport_to_world_2d(camera_transform, last_screen_pos)
            {
                transform.translation -= (current_world - last_world_reprojected).extend(0.0);
                *pan = CameraPanState::Panning {
                    last_screen_pos: current_screen,
                    source,
                };
            }
        }
    }
}

/// Deletes the selected structure block on Delete/Backspace or the
/// structure toolbar's own Delete button — mirrors
/// `edit_mode::handle_delete_selected`.
pub fn handle_delete_selected_structure_block(
    keys: Res<ButtonInput<KeyCode>>,
    delete_button: Query<&Interaction, (Changed<Interaction>, With<StructureDeleteButton>)>,
    mut commands: Commands,
    mut selected: ResMut<SelectedStructureBlock>,
) {
    let button_pressed = delete_button.iter().any(|i| *i == Interaction::Pressed);
    if !(keys.just_pressed(KeyCode::Delete)
        || keys.just_pressed(KeyCode::Backspace)
        || button_pressed)
    {
        return;
    }
    let Some(entity) = selected.0.take() else {
        return;
    };
    commands.entity(entity).despawn();
}

fn draw_structure_block_outline(gizmos: &mut Gizmos, cell: IVec2, color: Color) {
    let center = cell_to_world(cell) + STRUCTURE_SPACE_OFFSET;
    gizmos.rect_2d(
        Isometry2d::from_translation(center),
        Vec2::splat(GRID_CELL_SIZE + SELECTION_OUTLINE_MARGIN),
        color,
    );
}

/// Magenta outline around the selected block — mirrors
/// `edit_mode::render_selection_highlight`.
pub fn render_structure_selection_highlight(
    selected: Res<SelectedStructureBlock>,
    cells: Query<&StructureCell>,
    mut gizmos: Gizmos,
) {
    let Some(entity) = selected.0 else {
        return;
    };
    let Ok(cell) = cells.get(entity) else {
        return;
    };
    draw_structure_block_outline(&mut gizmos, cell.0, COLOR_SELECTION);
}

/// Dimmer cyan outline around whatever's under the cursor before it's
/// clicked — mirrors `edit_mode::render_hover_highlight`.
pub fn render_structure_hover_highlight(
    pointer_over_ui: Res<PointerOverUi>,
    armed: Res<ArmedStructureTool>,
    drag: Res<StructureDragState>,
    selected: Res<SelectedStructureBlock>,
    pointer: Res<PointerState>,
    blocks: Query<(Entity, &StructureCell)>,
    mut gizmos: Gizmos,
) {
    if pointer_over_ui.0 || armed.0.is_some() || !matches!(*drag, StructureDragState::Idle) {
        return;
    }
    let Some(world_pos) = pointer.world_pos else {
        return;
    };
    let cell = world_to_cell(world_pos - STRUCTURE_SPACE_OFFSET);
    let Some((entity, hovered)) = blocks.iter().find(|(_, c)| c.0 == cell) else {
        return;
    };
    if Some(entity) == selected.0 {
        return;
    }
    draw_structure_block_outline(&mut gizmos, hovered.0, COLOR_HOVER);
}

/// Re-tints every currently-placed Corps block from `ActiveStructureColor`
/// — Pin/Lamp blocks keep their own fixed color regardless. Deliberately
/// unconditional (no `is_changed()` guard): `apply_loaded_appearances`
/// (`rendering/appearance.rs`) replaces a Body block's whole `Sprite` —
/// resetting `color` to white — the moment its `structure_body.json`
/// appearance finishes loading asynchronously, which can land on any frame
/// well after the last color change. Re-applying every frame this system
/// runs (already gated to `ProjectView::ChipEdit`, cheap given how few
/// blocks a chip structure ever has) catches that reset instead of leaving
/// the block flashing white until the player happens to touch the palette
/// again — same reasoning as `sync_pin_colors` (`rendering/sync.rs`), which
/// has no guard of its own for the identical reason.
pub fn sync_structure_color(
    color: Res<ActiveStructureColor>,
    mut blocks: Query<(&StructureBlockKind, &mut Sprite)>,
) {
    for (kind, mut sprite) in &mut blocks {
        if *kind == StructureBlockKind::Body {
            sprite.color = color.0;
        }
    }
}

/// A visual-only leg sprite bridging a Pin block to an orthogonally-adjacent
/// Corps or Lampe block — see `sync_structure_pin_legs`.
#[derive(Component)]
pub(crate) struct StructurePinLeg;

/// Makes a "leg" sprite (same `pin_lead.json` art as `spawn::leg`) appear
/// between every Pin block and any Corps or Lampe block orthogonally
/// adjacent to it — same visual language as native gates' own pins (see
/// `spawn.rs`), but recomputed on change rather than fixed at spawn time,
/// since Pin, Corps and Lampe are independent blocks here that can each be
/// placed/dragged/deleted on their own. Despawns and fully respawns every leg
/// on each recompute rather than reconciling incrementally: a chip structure's block count is always
/// small, so there's no need for the extra complexity (same reasoning as
/// `sync_background_grid`'s pool, just skipped here since it doesn't pay for
/// itself at this scale). This full despawn also incidentally cleans up a
/// previous project's legs the moment a new project's blocks respawn
/// (triggering `Added<StructureCell>`), with no need to touch
/// `project::switch_to_project` itself.
pub fn sync_structure_pin_legs(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut removed: RemovedComponents<StructureCell>,
    changed: Query<(), Changed<StructureCell>>,
    blocks: Query<(&StructureCell, &StructureBlockKind)>,
    legs: Query<Entity, With<StructurePinLeg>>,
) {
    let removed_any = removed.read().count() > 0;
    if changed.is_empty() && !removed_any {
        return;
    }

    for entity in &legs {
        commands.entity(entity).despawn();
    }

    // A Pin attaches to either a Corps or a Lampe block — the two are the
    // only kinds a Pin ever needs to visually anchor to; a Pin never
    // attaches to another Pin.
    let attach_cells: Vec<IVec2> = blocks
        .iter()
        .filter(|(_, kind)| matches!(kind, StructureBlockKind::Body | StructureBlockKind::Lamp))
        .map(|(cell, _)| cell.0)
        .collect();

    const NEIGHBOR_OFFSETS: [IVec2; 4] = [
        IVec2::new(1, 0),
        IVec2::new(-1, 0),
        IVec2::new(0, 1),
        IVec2::new(0, -1),
    ];

    for (cell, kind) in &blocks {
        if !matches!(kind, StructureBlockKind::Pin) {
            continue;
        }
        for offset in NEIGHBOR_OFFSETS {
            let neighbor = cell.0 + offset;
            if !attach_cells.contains(&neighbor) {
                continue;
            }
            let mid_world =
                (cell_to_world(cell.0) + cell_to_world(neighbor)) / 2.0 + STRUCTURE_SPACE_OFFSET;
            // `pin_lead.json` is authored as a horizontal bar (the same
            // asset native gates use unrotated for their own left/right
            // legs — see `spawn::leg`); rotate a quarter turn for a
            // vertically-adjacent pin. The art is symmetric, so one
            // rotation covers both the up and down cases.
            let rotation = if offset.x != 0 {
                Quat::IDENTITY
            } else {
                Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)
            };
            commands.spawn((
                StructurePinLeg,
                Sprite::default(),
                PendingAppearance(asset_server.load("appearances/pin_lead.json")),
                Transform::from_translation(mid_world.extend(-0.05)).with_rotation(rotation),
            ));
        }
    }
}

/// The chip's name, rendered in world-space above the topmost Corps block —
/// a single entity spawned once at `Startup` (`spawn_structure_name_label`)
/// and kept in sync in place by `sync_structure_name_label`. Unlike
/// `StructurePinLeg`, it has no lifecycle tied to `StructureCell` — nothing
/// to leak across a project switch — so there's no need to despawn/respawn
/// it at all, just reposition/retext/reshow it.
#[derive(Component)]
pub(crate) struct StructureNameLabel;

/// Extra world-space gap above the topmost Corps row so the label reads as
/// floating above the structure instead of touching it.
const STRUCTURE_LABEL_Y_PADDING: f32 = GRID_CELL_SIZE * 0.75;

/// Comfortably above every placed structure block's own z (which climbs
/// from 0.0 by `SPAWN_Z_STEP` per block) — the label sits well outside their
/// footprint in Y anyway, so this mostly just keeps it deterministic.
const STRUCTURE_LABEL_Z: f32 = 2.0;

pub fn spawn_structure_name_label(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        StructureNameLabel,
        Text2d::new(""),
        TextFont {
            font: asset_server.load(UI_FONT_PATH).into(),
            font_size: LABEL_FONT_SIZE.into(),
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::default(),
        Visibility::Hidden,
    ));
}

/// Positions, retexts and shows/hides the chip name label from
/// `ActiveStructureLabel` and the current Corps blocks. Deliberately
/// ungated by `ProjectView::ChipEdit` — same reasoning as
/// `sync_structure_pin_legs`: recomputing while hidden costs nothing, and a
/// `run_if` gate that happens to be off exactly when a project switch
/// despawns/respawns blocks would risk the same missed-frame class of bug
/// already hit (and fixed) there.
pub fn sync_structure_name_label(
    label: Res<ActiveStructureLabel>,
    mut removed: RemovedComponents<StructureCell>,
    changed: Query<(), Changed<StructureCell>>,
    blocks: Query<(&StructureCell, &StructureBlockKind)>,
    mut target: Query<(&mut Text2d, &mut Transform, &mut Visibility), With<StructureNameLabel>>,
) {
    let removed_any = removed.read().count() > 0;
    if !label.is_changed() && changed.is_empty() && !removed_any {
        return;
    }
    let Ok((mut text, mut transform, mut visibility)) = target.single_mut() else {
        return;
    };

    let mut corps_cells = blocks
        .iter()
        .filter(|(_, kind)| matches!(kind, StructureBlockKind::Body))
        .map(|(cell, _)| cell.0);
    let Some(first) = corps_cells.next() else {
        *visibility = Visibility::Hidden;
        return;
    };
    let (min_x, max_x, max_y) = corps_cells.fold(
        (first.x, first.x, first.y),
        |(min_x, max_x, max_y), cell| (min_x.min(cell.x), max_x.max(cell.x), max_y.max(cell.y)),
    );

    if label.0.is_empty() {
        *visibility = Visibility::Hidden;
        return;
    }

    text.0 = label.0.clone();
    let center_x = (min_x + max_x) as f32 / 2.0 * GRID_CELL_SIZE;
    let top_y = max_y as f32 * GRID_CELL_SIZE;
    let position = Vec2::new(center_x, top_y + STRUCTURE_LABEL_Y_PADDING) + STRUCTURE_SPACE_OFFSET;
    transform.translation = position.extend(STRUCTURE_LABEL_Z);
    *visibility = Visibility::Visible;
}

/// The selected structure block's detail panel — same principle (and
/// position/style) as `inspector::InspectorPanel` for the interior circuit:
/// fixed bottom-right, shown while any structure block is selected. A
/// separate panel rather than reusing `InspectorPanel` itself, since this one
/// also needs an editable Nom field for a Pin — but `spawn_structure_block_panel`
/// mirrors `spawn_inspector_panel`'s exact `Node` styling, and its Description
/// row is plain read-only `Text` just like `InspectorDescription`. **One
/// persistent entity**, spawned once at `Startup`, like `StructureNameLabel`:
/// no lifecycle tied to `StructureCell`, so no despawn/respawn risk — just
/// shown/hidden in place.
#[derive(Component)]
pub(crate) struct StructureBlockPanel;

/// Wraps the Nom caption + field + suggestion list so `sync_structure_block_panel`
/// can hide the whole section in one place for a Corps selection — only a
/// Pin or a Lamp carries `StructurePinLabel` (both are connection points
/// meant to link to an interior circuit pin by matching label; a Corps has
/// nothing to link), so Corps has nothing for this section to show.
#[derive(Component)]
pub(crate) struct StructurePinNameSection;

/// The clickable label field — clicking it toggles `StructurePinLabelFocus`.
#[derive(Component)]
pub(crate) struct StructurePinLabelField;

/// The `Text` child of `StructurePinLabelField` kept in sync with the
/// selected Pin/Lamp's `StructurePinLabel` by `sync_structure_pin_label_field_text`.
#[derive(Component)]
pub(crate) struct StructurePinLabelFieldText;

/// Container for the dynamically rebuilt list of existing-label suggestion
/// buttons — see `sync_structure_pin_label_suggestions`.
#[derive(Component)]
pub(crate) struct StructurePinLabelSuggestions;

/// One suggestion button, carrying the label it applies when clicked.
#[derive(Component, Clone)]
pub(crate) struct StructurePinLabelSuggestionButton(pub String);

/// The `Text` node showing the selected block's fixed
/// `structure_block_description` — kept in sync by `sync_structure_block_panel`.
#[derive(Component)]
pub(crate) struct StructureDescriptionText;

/// Shows "Lié" once the selected Pin/Lamp's label matches another
/// `StructurePinLabel`-carrying entity anywhere — including a `PinHeader` in
/// the interior circuit (`editor::pin_header`), since matching labels across
/// the two views is exactly what "linked" means. Plain text, no icon (see
/// the glyph-coverage lesson already documented elsewhere in this file).
#[derive(Component)]
pub(crate) struct StructureLinkedText;

const STRUCTURE_PIN_LABEL_PLACEHOLDER: &str = "Label de connexion";

fn structure_pin_panel_caption(font: Handle<Font>, text: &str) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font: font.into(),
            font_size: LABEL_FONT_SIZE.into(),
            ..default()
        },
        TextColor(Color::WHITE),
    )
}

/// Same `Node`/color styling as `inspector::spawn_inspector_panel` — bottom
/// right, fixed, so the structure block detail panel reads as the same UI
/// language as the interior circuit's own selection panel.
pub fn spawn_structure_block_panel(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT_PATH);
    commands
        .spawn((
            StructureBlockPanel,
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                right: Val::Px(10.0),
                width: Val::Px(240.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(COLOR_BUTTON_NORMAL),
            BorderColor::all(COLOR_BUTTON_BORDER),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    StructurePinNameSection,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                ))
                .with_children(|parent| {
                    parent.spawn(structure_pin_panel_caption(font.clone(), "Nom"));
                    parent.spawn((
                        Button,
                        StructurePinLabelField,
                        Node {
                            height: Val::Px(32.0),
                            justify_content: JustifyContent::FlexStart,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(Val::Px(6.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(COLOR_BUTTON_NORMAL),
                        BorderColor::all(COLOR_BUTTON_BORDER),
                        children![(
                            StructurePinLabelFieldText,
                            Text::new(STRUCTURE_PIN_LABEL_PLACEHOLDER),
                            TextFont {
                                font: font.clone().into(),
                                font_size: LABEL_FONT_SIZE.into(),
                                ..default()
                            },
                            TextColor(COLOR_BUTTON_BORDER),
                        )],
                    ));
                    parent.spawn((
                        StructurePinLabelSuggestions,
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(2.0),
                            ..default()
                        },
                    ));
                    parent.spawn((
                        StructureLinkedText,
                        Text::new(""),
                        inspector_text_font(font.clone()),
                    ));
                });
            parent.spawn(structure_pin_panel_caption(font.clone(), "Description"));
            parent.spawn((
                StructureDescriptionText,
                Text::new(""),
                inspector_text_font(font),
            ));
        });
}

/// Same `TextFont`/`TextColor` pairing as `inspector::inspector_text_font` —
/// kept as a local duplicate rather than a shared import since the two
/// panels are otherwise independent and this is a two-line bundle.
fn inspector_text_font(font: Handle<Font>) -> impl Bundle {
    (
        TextFont {
            font: font.into(),
            font_size: LABEL_FONT_SIZE.into(),
            ..default()
        },
        TextColor(Color::WHITE),
    )
}

/// Shows/hides the structure block detail panel from `SelectedStructureBlock`,
/// fills in the fixed `structure_block_description` for whichever kind is
/// selected, and toggles the editable Nom section on/off (only a Pin or a
/// Lamp has a `StructurePinLabel` for it to edit) — same `selected`-change-gated
/// pattern as `inspector::sync_inspector_panel` (position is fixed, so unlike
/// the panel's previous floating design there's no per-frame reprojection
/// needed).
#[allow(clippy::too_many_arguments)]
pub fn sync_structure_block_panel(
    selected: Res<SelectedStructureBlock>,
    blocks: Query<&StructureBlockKind>,
    labels: Query<(Entity, &StructurePinLabel)>,
    mut panel: Query<&mut Node, With<StructureBlockPanel>>,
    mut name_section: Query<
        &mut Node,
        (With<StructurePinNameSection>, Without<StructureBlockPanel>),
    >,
    mut description_text: Query<&mut Text, With<StructureDescriptionText>>,
    mut linked_text: Query<
        &mut Text,
        (With<StructureLinkedText>, Without<StructureDescriptionText>),
    >,
) {
    if !selected.is_changed() {
        return;
    }
    let Ok(mut node) = panel.single_mut() else {
        return;
    };
    let kind = selected.0.and_then(|entity| blocks.get(entity).ok());
    node.display = if kind.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    let Some(kind) = kind else {
        return;
    };
    if let Ok(mut section) = name_section.single_mut() {
        section.display = if matches!(kind, StructureBlockKind::Pin | StructureBlockKind::Lamp) {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok(mut text) = description_text.single_mut() {
        text.0 = structure_block_description(*kind).to_string();
    }
    // Gated on `selected.is_changed()` like the rest of this system, so
    // typing a matching label on the *other* side won't flip this to "Lié"
    // until this panel's own selection changes again (re-click to refresh)
    // — acceptable for a first pass, same simplicity trade-off as the rest
    // of this incrementally-built link system.
    if let Ok(mut text) = linked_text.single_mut() {
        let own_label = selected.0.and_then(|entity| labels.get(entity).ok());
        let is_linked = own_label.is_some_and(|(entity, label)| {
            !label.0.is_empty()
                && labels
                    .iter()
                    .any(|(other, other_label)| other != entity && other_label.0 == label.0)
        });
        text.0 = if is_linked {
            "Lié".to_string()
        } else {
            String::new()
        };
    }
}

/// Same bundle shape as `hud::hud_button`, kept local since that one isn't
/// `pub(crate)` — same precedent already established in `sidebar.rs`.
fn structure_button_frame(font: Handle<Font>, label: &str) -> impl Bundle {
    (
        Node {
            width: Val::Px(72.0),
            height: Val::Px(36.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(COLOR_BUTTON_NORMAL),
        BorderColor::all(COLOR_BUTTON_BORDER),
        children![(
            Text::new(label),
            TextFont {
                font: font.into(),
                font_size: LABEL_FONT_SIZE.into(),
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    )
}

/// Spawns the (initially hidden) structure-editor toolbar — 3 tool buttons
/// (Corps/Pin/Lampe) + a row of fixed color swatches — bottom-left, same
/// spot and visual language as `hud::spawn_toolbar`. Shown/hidden by
/// `sync_structure_toolbar_visibility` in lockstep with `ProjectView`.
pub fn spawn_structure_toolbar(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT_PATH);
    commands
        .spawn((
            StructureToolbar,
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(10.0),
                column_gap: Val::Px(8.0),
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Button,
                StructureToolButton(StructureBlockKind::Body),
                structure_button_frame(font.clone(), "Corps"),
            ));
            parent.spawn((
                Button,
                StructureToolButton(StructureBlockKind::Pin),
                structure_button_frame(font.clone(), "Pin"),
            ));
            parent.spawn((
                Button,
                StructureToolButton(StructureBlockKind::Lamp),
                structure_button_frame(font.clone(), "Lampe"),
            ));
            for &swatch_color in STRUCTURE_COLOR_PALETTE.iter() {
                parent.spawn((
                    Button,
                    StructureColorButton(swatch_color),
                    Node {
                        width: Val::Px(28.0),
                        height: Val::Px(28.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(swatch_color),
                    BorderColor::all(COLOR_BUTTON_BORDER),
                ));
            }
            parent.spawn((
                Button,
                StructureDeleteButton,
                structure_button_frame(font.clone(), "Delete"),
            ));
            parent.spawn((
                Button,
                StructureLabelField,
                Node {
                    width: Val::Px(140.0),
                    height: Val::Px(36.0),
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(8.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(COLOR_BUTTON_NORMAL),
                BorderColor::all(COLOR_BUTTON_BORDER),
                children![(
                    StructureLabelText,
                    Text::new(STRUCTURE_LABEL_PLACEHOLDER),
                    TextFont {
                        font: font.into(),
                        font_size: LABEL_FONT_SIZE.into(),
                        ..default()
                    },
                    TextColor(COLOR_BUTTON_BORDER),
                )],
            ));
        });
}

pub fn sync_structure_toolbar_visibility(
    view: Res<ProjectView>,
    mut toolbar: Query<&mut Node, With<StructureToolbar>>,
) {
    if !view.is_changed() {
        return;
    }
    let Ok(mut node) = toolbar.single_mut() else {
        return;
    };
    node.display = if *view == ProjectView::ChipEdit {
        Display::Flex
    } else {
        Display::None
    };
}

/// Arms/disarms a structure tool — pressing the already-armed tool's own
/// button disarms it (there's no keyboard-independent way to reach
/// select/move/delete mode otherwise, unlike the interior editor's number
/// keys); `Escape` also disarms, mirroring
/// `placement::handle_tool_arming`'s `0`/`Escape` convention.
pub fn handle_structure_tool_button_click(
    keys: Res<ButtonInput<KeyCode>>,
    mut armed: ResMut<ArmedStructureTool>,
    buttons: Query<(&Interaction, &StructureToolButton), Changed<Interaction>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        armed.0 = None;
        return;
    }
    for (interaction, button) in &buttons {
        if *interaction == Interaction::Pressed {
            armed.0 = if armed.0 == Some(button.0) {
                None
            } else {
                Some(button.0)
            };
        }
    }
}

pub fn handle_structure_color_button_click(
    mut active: ResMut<ActiveStructureColor>,
    buttons: Query<(&Interaction, &StructureColorButton), Changed<Interaction>>,
) {
    for (interaction, button) in &buttons {
        if *interaction == Interaction::Pressed {
            active.0 = button.0;
        }
    }
}

pub fn sync_structure_toolbar_highlight(
    armed: Res<ArmedStructureTool>,
    active_color: Res<ActiveStructureColor>,
    mut tool_buttons: Query<
        (&StructureToolButton, &mut BorderColor),
        Without<StructureColorButton>,
    >,
    mut color_buttons: Query<
        (&StructureColorButton, &mut BorderColor),
        Without<StructureToolButton>,
    >,
) {
    if !armed.is_changed() && !active_color.is_changed() {
        return;
    }
    for (button, mut border) in &mut tool_buttons {
        *border = BorderColor::all(if armed.0 == Some(button.0) {
            COLOR_BUTTON_ARMED
        } else {
            COLOR_BUTTON_BORDER
        });
    }
    for (button, mut border) in &mut color_buttons {
        *border = BorderColor::all(if button.0 == active_color.0 {
            COLOR_BUTTON_ARMED
        } else {
            COLOR_BUTTON_BORDER
        });
    }
}

/// Focuses the name field on click; defocuses it on Entrée/Échap, or on any
/// other click (another toolbar button, or the canvas itself) — a stuck
/// focus would otherwise swallow Delete/Backspace into the label text
/// instead of letting `handle_delete_selected_structure_block` see them.
pub fn handle_structure_label_field_click(
    keys: Res<ButtonInput<KeyCode>>,
    pointer: Res<PointerState>,
    pointer_over_ui: Res<PointerOverUi>,
    mut focus: ResMut<StructureLabelFocus>,
    field: Query<&Interaction, (Changed<Interaction>, With<StructureLabelField>)>,
    other_buttons: Query<&Interaction, (Changed<Interaction>, Without<StructureLabelField>)>,
) {
    if field.iter().any(|i| *i == Interaction::Pressed) {
        focus.0 = true;
        return;
    }
    if !focus.0 {
        return;
    }
    let clicked_elsewhere_in_ui = other_buttons.iter().any(|i| *i == Interaction::Pressed);
    let clicked_canvas = pointer.just_pressed && !pointer_over_ui.0;
    if clicked_elsewhere_in_ui
        || clicked_canvas
        || keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::Escape)
    {
        focus.0 = false;
    }
}

/// Appends typed characters to `ActiveStructureLabel` while the name field
/// is focused. Drains (rather than ignores) `KeyboardInput` while unfocused
/// — `MessageReader`'s cursor otherwise keeps advancing through a growing
/// backlog, and the next focus would replay every keystroke typed while
/// unfocused in one burst.
pub fn handle_structure_label_typing(
    focus: Res<StructureLabelFocus>,
    mut keys: MessageReader<KeyboardInput>,
    mut label: ResMut<ActiveStructureLabel>,
) {
    if !focus.0 {
        keys.clear();
        return;
    }
    for event in keys.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match event.key_code {
            KeyCode::Backspace => {
                label.0.pop();
            }
            KeyCode::Enter | KeyCode::Escape => {}
            _ => {
                if let Some(text) = &event.text {
                    label.0.push_str(text);
                }
            }
        }
    }
}

/// Keeps the toolbar field's displayed text in sync with
/// `ActiveStructureLabel`, falling back to `STRUCTURE_LABEL_PLACEHOLDER`
/// when empty.
pub fn sync_structure_label_field_text(
    label: Res<ActiveStructureLabel>,
    mut text: Query<&mut Text, With<StructureLabelText>>,
) {
    if !label.is_changed() {
        return;
    }
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    text.0 = if label.0.is_empty() {
        STRUCTURE_LABEL_PLACEHOLDER.to_string()
    } else {
        label.0.clone()
    };
}

/// Highlights the name field's border while it holds keyboard focus — same
/// `COLOR_BUTTON_ARMED`/`COLOR_BUTTON_BORDER` convention as an armed
/// structure tool.
pub fn sync_structure_label_field_border(
    focus: Res<StructureLabelFocus>,
    mut field: Query<&mut BorderColor, With<StructureLabelField>>,
) {
    if !focus.is_changed() {
        return;
    }
    let Ok(mut border) = field.single_mut() else {
        return;
    };
    *border = BorderColor::all(if focus.0 {
        COLOR_BUTTON_ARMED
    } else {
        COLOR_BUTTON_BORDER
    });
}

/// Focuses the floating per-pin label field on click; defocuses on
/// Entrée/Échap or any other click — same anti-collision-with-Delete
/// reasoning as `handle_structure_label_field_click`.
pub fn handle_structure_pin_label_field_click(
    keys: Res<ButtonInput<KeyCode>>,
    pointer: Res<PointerState>,
    pointer_over_ui: Res<PointerOverUi>,
    mut focus: ResMut<StructurePinLabelFocus>,
    field: Query<&Interaction, (Changed<Interaction>, With<StructurePinLabelField>)>,
    other_buttons: Query<&Interaction, (Changed<Interaction>, Without<StructurePinLabelField>)>,
) {
    if field.iter().any(|i| *i == Interaction::Pressed) {
        focus.0 = true;
        return;
    }
    if !focus.0 {
        return;
    }
    let clicked_elsewhere_in_ui = other_buttons.iter().any(|i| *i == Interaction::Pressed);
    let clicked_canvas = pointer.just_pressed && !pointer_over_ui.0;
    if clicked_elsewhere_in_ui
        || clicked_canvas
        || keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::Escape)
    {
        focus.0 = false;
    }
}

/// Appends typed characters to the *selected* Pin/Lamp's `StructurePinLabel`
/// while the floating field is focused — mirrors
/// `handle_structure_label_typing`, but mutates a per-entity component
/// instead of a global resource, so it needs to look up the target entity
/// each event batch. Still drains `KeyboardInput` whenever there's nothing
/// valid to type into (unfocused, or the selection isn't a Pin/Lamp anymore —
/// e.g. it was just deleted), for the same reason `handle_structure_label_typing`
/// always drains while unfocused.
pub fn handle_structure_pin_label_typing(
    focus: Res<StructurePinLabelFocus>,
    selected: Res<SelectedStructureBlock>,
    mut keys: MessageReader<KeyboardInput>,
    mut blocks: Query<(&StructureBlockKind, &mut StructurePinLabel)>,
) {
    let target = focus.0.then_some(selected.0).flatten().and_then(|entity| {
        blocks
            .get_mut(entity)
            .ok()
            .filter(|(kind, _)| matches!(kind, StructureBlockKind::Pin | StructureBlockKind::Lamp))
            .map(|(_, label)| label)
    });
    let Some(mut label) = target else {
        keys.clear();
        return;
    };
    for event in keys.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        match event.key_code {
            KeyCode::Backspace => {
                label.0.pop();
            }
            KeyCode::Enter | KeyCode::Escape => {}
            _ => {
                if let Some(text) = &event.text {
                    label.0.push_str(text);
                }
            }
        }
    }
}

/// Keeps the floating field's displayed text in sync with the selected
/// Pin/Lamp's `StructurePinLabel`, falling back to
/// `STRUCTURE_PIN_LABEL_PLACEHOLDER` when there's no Pin/Lamp selected or its
/// label is empty. Unconditional (no change-detection guard): cheap single
/// lookup, and the trigger set (selection changed vs. the selected block's
/// own label changed) isn't a single resource to gate on cleanly — same
/// reasoning `sync_pin_colors` (`rendering/sync.rs`) uses for having no
/// guard at all.
pub fn sync_structure_pin_label_field_text(
    selected: Res<SelectedStructureBlock>,
    blocks: Query<(&StructureBlockKind, &StructurePinLabel)>,
    mut text: Query<&mut Text, With<StructurePinLabelFieldText>>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let label = selected
        .0
        .and_then(|entity| blocks.get(entity).ok())
        .filter(|(kind, _)| matches!(kind, StructureBlockKind::Pin | StructureBlockKind::Lamp))
        .map(|(_, label)| label.0.clone());
    text.0 = match label {
        Some(label) if !label.is_empty() => label,
        _ => STRUCTURE_PIN_LABEL_PLACEHOLDER.to_string(),
    };
}

/// Highlights the floating field's border while it holds keyboard focus —
/// same convention as `sync_structure_label_field_border`.
pub fn sync_structure_pin_label_field_border(
    focus: Res<StructurePinLabelFocus>,
    mut field: Query<&mut BorderColor, With<StructurePinLabelField>>,
) {
    if !focus.is_changed() {
        return;
    }
    let Ok(mut border) = field.single_mut() else {
        return;
    };
    *border = BorderColor::all(if focus.0 {
        COLOR_BUTTON_ARMED
    } else {
        COLOR_BUTTON_BORDER
    });
}

/// Rebuilds the suggestion button list from every distinct, non-empty
/// `StructurePinLabel` found anywhere *other* than the block currently
/// selected — deliberately not filtered to `StructureBlockKind::Pin | Lamp`
/// (that would be redundant anyway: a Corps never carries this component)
/// so the pool also picks up labels from the interior circuit's `PinHeader`
/// entities (`editor::pin_header`) — matching labels across the structure
/// and interior views is exactly what marks them as linked. Cached in `last`
/// and only actually despawned/respawned when the computed set changes, so
/// typing into the selected block's own field (which doesn't affect this
/// set, since it's excluded) never churns the UI. Despawn-and-respawn rather
/// than incremental diffing, same reasoning as `sync_structure_pin_legs`: a
/// chip's pin/lamp count is always small.
pub fn sync_structure_pin_label_suggestions(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    selected: Res<SelectedStructureBlock>,
    blocks: Query<(Entity, &StructurePinLabel)>,
    suggestions_root: Query<Entity, With<StructurePinLabelSuggestions>>,
    existing_buttons: Query<Entity, With<StructurePinLabelSuggestionButton>>,
    mut last: Local<Vec<String>>,
) {
    let mut current: Vec<String> = blocks
        .iter()
        .filter(|(entity, label)| Some(*entity) != selected.0 && !label.0.is_empty())
        .map(|(_, label)| label.0.clone())
        .collect();
    current.sort();
    current.dedup();

    if *last == current {
        return;
    }
    *last = current.clone();

    for entity in &existing_buttons {
        commands.entity(entity).despawn();
    }
    let Ok(root) = suggestions_root.single() else {
        return;
    };
    let font = asset_server.load(UI_FONT_PATH);
    commands.entity(root).with_children(|parent| {
        for label in current {
            parent.spawn((
                Button,
                StructurePinLabelSuggestionButton(label.clone()),
                Node {
                    width: Val::Px(120.0),
                    height: Val::Px(24.0),
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(COLOR_BUTTON_NORMAL),
                BorderColor::all(COLOR_BUTTON_BORDER),
                children![(
                    Text::new(label),
                    TextFont {
                        font: font.clone().into(),
                        font_size: LABEL_FONT_SIZE.into(),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                )],
            ));
        }
    });
}

/// Clicking a suggestion writes its label directly onto the selected
/// Pin/Lamp's `StructurePinLabel`, no typing needed.
pub fn handle_structure_pin_label_suggestion_click(
    selected: Res<SelectedStructureBlock>,
    buttons: Query<(&Interaction, &StructurePinLabelSuggestionButton), Changed<Interaction>>,
    mut blocks: Query<(&StructureBlockKind, &mut StructurePinLabel)>,
) {
    let Some(entity) = selected.0 else {
        return;
    };
    let Some((_, clicked)) = buttons.iter().find(|(i, _)| **i == Interaction::Pressed) else {
        return;
    };
    let Ok((kind, mut label)) = blocks.get_mut(entity) else {
        return;
    };
    if !matches!(kind, StructureBlockKind::Pin | StructureBlockKind::Lamp) {
        return;
    }
    label.0 = clicked.0.clone();
}
