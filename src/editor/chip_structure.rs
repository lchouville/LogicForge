use bevy::prelude::*;

use crate::constants::{
    COLOR_BUTTON_ARMED, COLOR_BUTTON_BORDER, COLOR_BUTTON_NORMAL, COLOR_HOVER, COLOR_NEUTRAL,
    COLOR_SELECTION, COLOR_STRUCTURE_LAMP, EDIT_DRAG_THRESHOLD, GRID_CELL_SIZE, LABEL_FONT_SIZE,
    SELECTION_OUTLINE_MARGIN, SPAWN_Z_STEP, STRUCTURE_COLOR_PALETTE, STRUCTURE_SPACE_OFFSET,
    UI_FONT_PATH,
};
use crate::grid::{cell_to_world, world_to_cell};
use crate::rendering::appearance::PendingAppearance;

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

#[derive(Component)]
pub(crate) struct StructureToolbar;

#[derive(Component, Clone, Copy)]
pub(crate) struct StructureToolButton(StructureBlockKind);

#[derive(Component, Clone, Copy)]
pub(crate) struct StructureColorButton(Color);

/// Touch/click equivalent of `Delete`/`Backspace` — mirrors `hud::DeleteButton`.
#[derive(Component)]
pub(crate) struct StructureDeleteButton;

/// Spawns one 1x1 structure block. `Pin` reuses the exact same visual as an
/// interior pin (`spawn::pin`'s `pin.json` appearance, same
/// `COLOR_NEUTRAL` placeholder while it loads) rather than a flat placeholder
/// square, so it reads as the same "connection point" the player already
/// recognizes from the interior circuit editor. `Body`/`Lamp` have no
/// equivalent art yet, so they stay flat-colored.
pub fn spawn_structure_block(
    commands: &mut Commands,
    asset_server: &AssetServer,
    cell: IVec2,
    kind: StructureBlockKind,
    body_color: Color,
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
            entity.insert(placeholder_sprite(body_color, 1.0, 1.0));
        }
        StructureBlockKind::Pin => {
            entity.insert((
                placeholder_sprite(COLOR_NEUTRAL, 1.0, 1.0),
                PendingAppearance(asset_server.load("appearances/pin.json")),
            ));
        }
        StructureBlockKind::Lamp => {
            entity.insert(placeholder_sprite(COLOR_STRUCTURE_LAMP, 1.0, 1.0));
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
            spawn_structure_block(&mut commands, &asset_server, cell, kind, color.0, z);
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

/// Re-tints every currently-placed Corps block when `ActiveStructureColor`
/// changes — Pin/Lamp blocks keep their own fixed color regardless.
pub fn sync_structure_color(
    color: Res<ActiveStructureColor>,
    mut blocks: Query<(&StructureBlockKind, &mut Sprite)>,
) {
    if !color.is_changed() {
        return;
    }
    for (kind, mut sprite) in &mut blocks {
        if *kind == StructureBlockKind::Body {
            sprite.color = color.0;
        }
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
                structure_button_frame(font, "Delete"),
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
