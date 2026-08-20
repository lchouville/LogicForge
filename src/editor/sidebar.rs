use bevy::prelude::*;

use crate::constants::{
    COLOR_BUTTON_ARMED, COLOR_BUTTON_BORDER, COLOR_BUTTON_NORMAL, LABEL_FONT_SIZE, SIDEBAR_WIDTH,
    UI_FONT_PATH,
};
use crate::simulation::components::{Cable, GateKind, GridPosition, Lamp, Switch};

use super::chip_structure::{ActiveStructureColor, StructureBlockKind, StructureCell};
use super::project::{
    CircuitEntityFilter, ProjectId, ProjectLibrary, ViewSwitchState, switch_to_project,
};
use super::resources::TransientEditorState;

/// The active `Camera2d`, provably disjoint (see `Without`) from every
/// circuit-entity query in `handle_project_selection` so Bevy can tell its
/// `&mut Transform` access can never alias theirs.
type ActiveCameraFilter = (With<Camera2d>, Without<GridPosition>, Without<Cable>);

#[derive(Resource)]
pub struct SidebarOpen(pub bool);

impl Default for SidebarOpen {
    fn default() -> Self {
        SidebarOpen(true)
    }
}

#[derive(Component)]
pub struct SidebarToggleButton;

#[derive(Component)]
pub struct NewProjectButton;

#[derive(Component, Clone, Copy)]
pub struct ProjectRow(pub ProjectId);

#[derive(Component)]
pub(crate) struct SidebarBody;

#[derive(Component)]
pub(crate) struct ProjectRowsContainer;

fn sidebar_text_font(font: Handle<Font>) -> impl Bundle {
    (
        TextFont {
            font: font.into(),
            font_size: LABEL_FONT_SIZE.into(),
            ..default()
        },
        TextColor(Color::WHITE),
    )
}

/// A plain bordered row button shared by the "+ Nouveau projet" button and
/// every project row — same shape as `hud.rs::hud_button`, kept local since
/// that one isn't `pub(crate)`.
fn sidebar_row_button(label: &str, background: Color, font: Handle<Font>) -> impl Bundle {
    (
        Node {
            padding: UiRect::all(Val::Px(6.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(background),
        BorderColor::all(COLOR_BUTTON_BORDER),
        children![(Text::new(label), sidebar_text_font(font))],
    )
}

/// Spawns the always-visible collapse/expand handle and the fully
/// collapsible project-list body as separate siblings — the outer container
/// itself has no background and hugs its content height (no `bottom: 0`),
/// so when `SidebarBody` is hidden nothing but the small handle remains: no
/// full-height colored bar left behind to overlap whatever's under it. See
/// `sync_sidebar_collapse` for how the body's `Display` follows
/// `SidebarOpen`, and `sync_project_rows` for how its row list is
/// (re)populated from `ProjectLibrary`.
pub fn spawn_sidebar(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT_PATH);
    commands
        .spawn((Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },))
        .with_children(|parent| {
            // Plain text, not a hamburger icon (was `\u{2630}`):
            // `FiraMono-Medium` only covers Latin, that glyph rendered as an
            // empty missing-glyph box — confirmed visually in a browser
            // build. Same fix as `inspector.rs`'s loupe placeholder.
            parent.spawn((
                Button,
                SidebarToggleButton,
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
                children![(Text::new("Projets"), sidebar_text_font(font.clone()))],
            ));
            parent.spawn((
                SidebarBody,
                Node {
                    display: Display::None,
                    width: Val::Px(SIDEBAR_WIDTH),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(6.0)),
                    row_gap: Val::Px(6.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(COLOR_BUTTON_NORMAL),
                BorderColor::all(COLOR_BUTTON_BORDER),
                children![
                    (
                        Button,
                        NewProjectButton,
                        sidebar_row_button("+ Nouveau projet", COLOR_BUTTON_NORMAL, font.clone()),
                    ),
                    (
                        ProjectRowsContainer,
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            ..default()
                        },
                    ),
                ],
            ));
        });
}

/// Rebuilds every project row from scratch whenever `ProjectLibrary` changes
/// (new project created, or the active one switched) — same "despawn and
/// respawn from scratch, cheap at this scale" precedent as
/// `rendering::cable::rebuild_cable_segments`.
pub fn sync_project_rows(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    library: Res<ProjectLibrary>,
    container: Query<(Entity, Option<&Children>), With<ProjectRowsContainer>>,
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
            let background = if entry.id == library.active {
                COLOR_BUTTON_ARMED
            } else {
                COLOR_BUTTON_NORMAL
            };
            parent.spawn((
                Button,
                ProjectRow(entry.id),
                sidebar_row_button(&entry.name, background, font.clone()),
            ));
        }
    });
}

/// Handles both "+ Nouveau projet" and clicking an existing row in one
/// system so the actual switch (snapshot/despawn/respawn, see
/// `project::switch_to_project`) lives in exactly one call site.
#[allow(clippy::too_many_arguments)]
pub fn handle_project_selection(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut library: ResMut<ProjectLibrary>,
    rows: Query<(&Interaction, &ProjectRow), Changed<Interaction>>,
    new_project_button: Query<&Interaction, (Changed<Interaction>, With<NewProjectButton>)>,
    gates: Query<(&GateKind, &GridPosition, &Transform)>,
    switches: Query<(&Switch, &GridPosition, &Transform)>,
    lamps: Query<(&Lamp, &GridPosition, &Transform)>,
    cables: Query<&Cable>,
    despawn_targets: Query<Entity, CircuitEntityFilter>,
    structure_blocks: Query<(&StructureBlockKind, &StructureCell)>,
    structure_despawn_targets: Query<Entity, With<StructureCell>>,
    mut active_structure_color: ResMut<ActiveStructureColor>,
    mut camera: Single<(&mut Transform, &mut Projection), ActiveCameraFilter>,
    mut state: TransientEditorState,
    mut view_switch: ViewSwitchState,
) {
    let target = if new_project_button
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        Some(library.create_project())
    } else {
        rows.iter()
            .find(|(interaction, _)| **interaction == Interaction::Pressed)
            .map(|(_, row)| row.0)
    };
    let Some(target) = target else {
        return;
    };

    let (camera_transform, projection) = &mut *camera;
    switch_to_project(
        target,
        &mut library,
        &mut commands,
        &asset_server,
        &gates,
        &switches,
        &lamps,
        &cables,
        &despawn_targets,
        &structure_blocks,
        &structure_despawn_targets,
        &mut active_structure_color,
        camera_transform,
        projection,
        &mut state,
        &mut view_switch,
    );
}

pub fn handle_sidebar_toggle_click(
    mut open: ResMut<SidebarOpen>,
    toggle_button: Query<&Interaction, (Changed<Interaction>, With<SidebarToggleButton>)>,
) {
    if toggle_button
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        open.0 = !open.0;
    }
}

pub fn sync_sidebar_collapse(
    open: Res<SidebarOpen>,
    mut body: Query<&mut Node, With<SidebarBody>>,
) {
    if !open.is_changed() {
        return;
    }
    let Ok(mut node) = body.single_mut() else {
        return;
    };
    node.display = if open.0 { Display::Flex } else { Display::None };
}
