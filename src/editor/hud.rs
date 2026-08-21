use bevy::prelude::*;

use crate::constants::{
    COLOR_BUTTON_ARMED, COLOR_BUTTON_BORDER, COLOR_BUTTON_NORMAL, LABEL_FONT_SIZE, UI_FONT_PATH,
};
use crate::simulation::components::GateKind;

use super::project::ProjectView;
use super::resources::{ArmedTool, Mode, PendingRotation, ToolKind};

#[derive(Component, Clone, Copy)]
pub struct ToolButton(pub ToolKind);

/// Touch/click equivalent of the `Tab` key — mirrors `toggle_mode`. Shows
/// the *current* mode as its own icon (see `sync_mode_button_icon`) rather
/// than a separate text label, since the selection/hover outlines already
/// make the active mode legible without one.
#[derive(Component)]
pub struct ModeToggleButton;

/// The icon `Text` child of `ModeToggleButton` — kept as its own marker so
/// `sync_mode_button_icon` can target just that one child, not the button's
/// whole subtree.
#[derive(Component)]
pub(crate) struct ModeToggleButtonIcon;

/// Touch/click equivalent of `R`/the right arrow — mirrors
/// `handle_selected_rotation`'s clockwise step. No counter-clockwise button:
/// three clockwise presses reach the same state.
#[derive(Component)]
pub struct RotateButton;

/// Touch/click equivalent of `Delete`/`Backspace` — mirrors
/// `handle_delete_selected`.
#[derive(Component)]
pub struct DeleteButton;

/// Switches the active project between its standard circuit editor and the
/// chip structure editor — see `chip_view.rs`/`chip_structure.rs`.
#[derive(Component)]
pub struct ChipViewToggleButton;

/// Marks the two standard-editor-only panels (component hotbar + action
/// bar) so `sync_standard_ui_visibility` can hide them while the chip
/// structure editor is showing — mirrors
/// `chip_structure::StructureToolbar`'s own visibility, kept in lockstep
/// with the same `ProjectView` so the two views' UI never overlaps.
#[derive(Component)]
pub(crate) struct StandardEditorUi;

/// True while the cursor is over any UI element (toolbar button, etc.) this
/// frame, so world-click handlers (place/toggle/wire/edit) know to skip and
/// let the UI's own click handling own the interaction instead.
#[derive(Resource, Default)]
pub struct PointerOverUi(pub bool);

/// Component tool hotbar — bottom-left. Action buttons (Mode/Rotate/Delete/
/// chip-view toggle) aren't components, so they get their own bar — see
/// `spawn_action_bar`.
pub fn spawn_toolbar(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT_PATH);
    commands.spawn((
        StandardEditorUi,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            tool_button(ToolKind::Gate(GateKind::And), "1: AND", font.clone()),
            tool_button(ToolKind::Gate(GateKind::Or), "2: OR", font.clone()),
            tool_button(ToolKind::Gate(GateKind::Not), "3: NOT", font.clone()),
            tool_button(ToolKind::Switch, "4: Switch", font.clone()),
            tool_button(ToolKind::Lamp, "5: Lamp", font.clone()),
            tool_button(ToolKind::Cable, "6: Cable", font.clone()),
            tool_button(ToolKind::Pin, "7: Pin", font),
        ],
    ));
}

/// Non-component action buttons (mode toggle, rotate, delete) — top-right,
/// separate from the component hotbar (`spawn_toolbar`) and clear of the
/// project sidebar (left edge). Standard-editor-only (`StandardEditorUi`),
/// unlike the chip-view toggle itself — see `spawn_view_toggle_button`.
pub fn spawn_action_bar(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT_PATH);
    commands.spawn((
        StandardEditorUi,
        Node {
            position_type: PositionType::Absolute,
            // Below `spawn_view_toggle_button`'s own row (36px tall + 8px
            // gap), so the two never overlap when both are visible.
            top: Val::Px(54.0),
            right: Val::Px(10.0),
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            (
                Button,
                ModeToggleButton,
                mode_button_frame(font.clone(), Mode::default()),
            ),
            (Button, RotateButton, hud_button("Rotate", font.clone())),
            (Button, DeleteButton, hud_button("Delete", font)),
        ],
    ));
}

/// The Standard <-> ChipEdit toggle, on its own — deliberately *not*
/// `StandardEditorUi`: unlike the rest of the action bar, this button must
/// stay visible in both views (it's the only way back from the chip
/// structure editor).
pub fn spawn_view_toggle_button(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT_PATH);
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        },
        children![(
            Button,
            ChipViewToggleButton,
            chip_view_toggle_button_frame(font, ProjectView::default()),
        )],
    ));
}

/// The label `Text` child of `ChipViewToggleButton` — kept as its own
/// marker so `sync_chip_view_toggle_label` can target just that child, same
/// reasoning as `ModeToggleButtonIcon`.
#[derive(Component)]
pub(crate) struct ChipViewToggleButtonLabel;

/// The label shown for the *current* view: "Vue structure" while in
/// Standard (press to go there), "Vue circuit intérieur" while in ChipEdit
/// (press to go back) — so the button always names where pressing it takes
/// you.
fn chip_view_toggle_label(view: ProjectView) -> &'static str {
    match view {
        ProjectView::Standard => "Vue structure",
        ProjectView::ChipEdit => "Vue circuit intérieur",
    }
}

fn chip_view_toggle_button_frame(font: Handle<Font>, initial_view: ProjectView) -> impl Bundle {
    (
        Node {
            // Wide enough for "Vue circuit intérieur" (22 chars) without
            // wrapping — narrower than this, Bevy UI text wraps rather than
            // overflowing, which broke this button's layout when it showed
            // the longer of its two labels.
            width: Val::Px(210.0),
            height: Val::Px(36.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(COLOR_BUTTON_NORMAL),
        BorderColor::all(COLOR_BUTTON_BORDER),
        children![(
            ChipViewToggleButtonLabel,
            Text::new(chip_view_toggle_label(initial_view)),
            TextFont {
                font: font.into(),
                font_size: LABEL_FONT_SIZE.into(),
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    )
}

pub fn sync_chip_view_toggle_label(
    view: Res<ProjectView>,
    mut label: Query<&mut Text, With<ChipViewToggleButtonLabel>>,
) {
    if !view.is_changed() {
        return;
    }
    let Ok(mut text) = label.single_mut() else {
        return;
    };
    text.0 = chip_view_toggle_label(*view).to_string();
}

/// Shared visual bundle for every HUD button (tool or action) — a plain
/// bordered box with a centered label, no `Button`/marker component of its
/// own so callers can attach whichever ones they need.
fn hud_button(label: &str, font: Handle<Font>) -> impl Bundle {
    (
        Node {
            width: Val::Px(84.0),
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

/// Same frame as `hud_button`, but its label carries `ModeToggleButtonIcon`
/// instead of being a plain unmarked `Text`, so `sync_mode_button_icon` can
/// find and update it as `Mode` changes.
fn mode_button_frame(font: Handle<Font>, initial_mode: Mode) -> impl Bundle {
    (
        Node {
            // Wide enough for "Interaction" (11 chars) without wrapping —
            // see `chip_view_toggle_button_frame`'s note on the same issue.
            width: Val::Px(110.0),
            height: Val::Px(36.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(COLOR_BUTTON_NORMAL),
        BorderColor::all(COLOR_BUTTON_BORDER),
        children![(
            ModeToggleButtonIcon,
            Text::new(mode_icon(initial_mode)),
            TextFont {
                font: font.into(),
                font_size: LABEL_FONT_SIZE.into(),
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    )
}

/// The label shown for the *current* mode: "Édition" while in Edit (press
/// to switch to Interaction), "Interaction" while in Interaction (press to
/// switch back to Edit). Plain text, not a pencil/eye icon as originally
/// built: `FiraMono-Medium` only covers Latin — the pencil/eye Unicode
/// (U+270F/U+1F441) rendered as empty missing-glyph boxes, confirmed
/// visually in a browser build. Same fix as the sidebar handle and the
/// inspector panel's loupe placeholder.
fn mode_icon(mode: Mode) -> &'static str {
    match mode {
        Mode::Edit => "Édition",
        Mode::Interaction => "Interaction",
    }
}

fn tool_button(tool: ToolKind, label: &str, font: Handle<Font>) -> impl Bundle {
    (Button, ToolButton(tool), hud_button(label, font))
}

pub fn handle_tool_button_click(
    mut armed: ResMut<ArmedTool>,
    mut rotation: ResMut<PendingRotation>,
    buttons: Query<(&Interaction, &ToolButton), Changed<Interaction>>,
) {
    for (interaction, tool_button) in &buttons {
        if *interaction == Interaction::Pressed {
            armed.0 = Some(tool_button.0);
            rotation.0 = 0;
        }
    }
}

pub fn sync_toolbar_highlight(
    armed: Res<ArmedTool>,
    mut buttons: Query<(&ToolButton, &mut BackgroundColor)>,
) {
    if !armed.is_changed() {
        return;
    }
    for (tool_button, mut background) in &mut buttons {
        background.0 = if armed.0 == Some(tool_button.0) {
            COLOR_BUTTON_ARMED
        } else {
            COLOR_BUTTON_NORMAL
        };
    }
}

pub fn update_pointer_over_ui(
    mut pointer: ResMut<PointerOverUi>,
    interactions: Query<&Interaction, With<Node>>,
) {
    pointer.0 = interactions
        .iter()
        .any(|interaction| *interaction != Interaction::None);
}

pub fn sync_mode_button_icon(
    mode: Res<Mode>,
    mut icon: Query<&mut Text, With<ModeToggleButtonIcon>>,
) {
    if !mode.is_changed() {
        return;
    }
    let Ok(mut text) = icon.single_mut() else {
        return;
    };
    text.0 = mode_icon(*mode).to_string();
}

/// Hides the standard editor's toolbar + action bar while the chip
/// structure editor is showing (`ProjectView::ChipEdit`), and vice versa —
/// the two views' bottom-left toolbars used to overlap since only the
/// structure one had this gating.
pub fn sync_standard_ui_visibility(
    view: Res<ProjectView>,
    mut panels: Query<&mut Node, With<StandardEditorUi>>,
) {
    if !view.is_changed() {
        return;
    }
    let display = if *view == ProjectView::Standard {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut panels {
        node.display = display;
    }
}
