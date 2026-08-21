use bevy::prelude::*;

use crate::constants::{
    COLOR_BUTTON_BORDER, COLOR_BUTTON_NORMAL, COLOR_NEUTRAL, COLOR_STRUCTURE_LAMP, LABEL_FONT_SIZE,
    PREVIEW_Z, UI_FONT_PATH,
};
use crate::grid::cell_to_world;
use crate::rendering::appearance::PendingAppearance;
use crate::simulation::components::GridPosition;

use super::chip_structure::StructureBlockKind;
use super::hud::StandardEditorUi;
use super::preview::{PlacementPreview, PlacementPreviewTint};
use super::project::{ProjectId, ProjectLibrary};
use super::resources::{ArmedTool, ToolKind};
use super::spawn::{facing_quat, placeholder_sprite};

/// A placed copy of another project's structure ("puce"), frozen at the
/// moment of placement — see `ProjectLibrary::chip_blueprint`. Purely
/// visual for now (roadmap item 5's next step is wiring real signal
/// continuity through it); `blocks` carries just enough to redraw itself
/// and to persist/respawn without ever looking `source` back up in
/// `ProjectLibrary` again, so a later rename/deletion of the source project
/// can't desync or break an already-placed instance.
#[derive(Component, Clone)]
pub struct ChipInstance {
    pub source: ProjectId,
    pub display_name: String,
    pub body_color: Color,
    pub blocks: Vec<(IVec2, StructureBlockKind)>,
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
) -> Entity {
    let world = cell_to_world(cell);
    let blocks = instance.blocks.clone();
    let body_color = instance.body_color;
    let mut root = commands.spawn((
        instance,
        GridPosition(cell),
        Transform::from_translation(world.extend(z)).with_rotation(facing_quat(rotation)),
        Visibility::default(),
    ));
    root.with_children(|parent| {
        for (local_cell, kind) in blocks {
            let offset = Transform::from_translation(cell_to_world(local_cell).extend(0.0));
            // Same per-kind appearance as `chip_structure::spawn_structure_block`
            // (Corps tinted by `body_color` with `structure_body.json`, Pin
            // with the shared `pin.json` socket art, Lampe a flat
            // `COLOR_STRUCTURE_LAMP` square, no async asset) but without
            // `StructureCell`/`StructurePinLabel` — a placed instance's
            // blocks are a read-only snapshot, not editable structure-editor
            // entities. Duplicated rather than reusing `spawn_structure_block`
            // directly since that function always spawns its own root entity
            // (never a child) and unconditionally tags components this copy
            // must not carry.
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
                    ));
                }
                StructureBlockKind::Lamp => {
                    parent.spawn((offset, placeholder_sprite(COLOR_STRUCTURE_LAMP, 1.0, 1.0)));
                }
            }
        }
    });
    root.id()
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
    blocks: &[(IVec2, StructureBlockKind)],
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
            for &(local_cell, kind) in blocks {
                let offset = Transform::from_translation(cell_to_world(local_cell).extend(0.0));
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
