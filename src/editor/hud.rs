use bevy::prelude::*;

use crate::constants::{
    COLOR_BUTTON_ARMED, COLOR_BUTTON_BORDER, COLOR_BUTTON_NORMAL, LABEL_FONT_SIZE,
};
use crate::simulation::components::GateKind;

use super::resources::{ArmedTool, Mode, ToolKind};

#[derive(Component)]
pub struct ModeLabel;

#[derive(Component, Clone, Copy)]
pub struct ToolButton(pub ToolKind);

/// True while the cursor is over any UI element (toolbar button, etc.) this
/// frame, so world-click handlers (place/toggle/wire/edit) know to skip and
/// let the UI's own click handling own the interaction instead.
#[derive(Resource, Default)]
pub struct PointerOverUi(pub bool);

pub fn spawn_mode_label(mut commands: Commands) {
    commands.spawn((
        ModeLabel,
        Text::new(mode_text(Mode::Interaction)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

pub fn spawn_toolbar(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            tool_button(ToolKind::Gate(GateKind::And), "1: AND"),
            tool_button(ToolKind::Gate(GateKind::Or), "2: OR"),
            tool_button(ToolKind::Gate(GateKind::Not), "3: NOT"),
            tool_button(ToolKind::Switch, "4: Switch"),
            tool_button(ToolKind::Lamp, "5: Lamp"),
        ],
    ));
}

fn tool_button(tool: ToolKind, label: &str) -> impl Bundle {
    (
        Button,
        ToolButton(tool),
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
                font_size: LABEL_FONT_SIZE.into(),
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    )
}

pub fn handle_tool_button_click(
    mut armed: ResMut<ArmedTool>,
    buttons: Query<(&Interaction, &ToolButton), Changed<Interaction>>,
) {
    for (interaction, tool_button) in &buttons {
        if *interaction == Interaction::Pressed {
            armed.0 = Some(tool_button.0);
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

pub fn sync_mode_label(mode: Res<Mode>, mut labels: Query<&mut Text, With<ModeLabel>>) {
    if !mode.is_changed() {
        return;
    }
    for mut text in &mut labels {
        text.0 = mode_text(*mode).to_string();
    }
}

fn mode_text(mode: Mode) -> &'static str {
    match mode {
        Mode::Interaction => {
            "Mode: Interaction — click a switch to toggle, drag pin to pin to wire (Tab to switch)"
        }
        Mode::Edit => {
            "Mode: Edit — drag a component to move it, click it to delete it (Tab to switch)"
        }
    }
}
