use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use crate::constants::{
    COLOR_BUTTON_ARMED, COLOR_BUTTON_BORDER, COLOR_BUTTON_NORMAL, LABEL_FONT_SIZE, UI_FONT_PATH,
};
use crate::simulation::components::PinHeader;

use super::chip_structure::StructurePinLabel;
use super::hud::PointerOverUi;
use super::pointer::PointerState;
use super::resources::Selected;

/// Whether the interior Pin's label field is currently capturing keystrokes
/// — mirrors `chip_structure::StructurePinLabelFocus`, kept as its own
/// resource since a focused structure-editor label field and a focused
/// interior one can never coexist (only one view is ever active) but are
/// otherwise fully independent state.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub struct PinHeaderLabelFocus(pub bool);

/// The selected `PinHeader`'s detail panel — same fixed bottom-right
/// principle as `inspector::InspectorPanel` and
/// `chip_structure::StructureBlockPanel`, but this one needs its own
/// editable Nom field (like the structure panel does for Pin/Lamp), so it's
/// a separate panel rather than a special case bolted onto `inspector.rs`'s
/// plain-`Text`-only one. A selected `PinHeader` never matches
/// `inspector::selected_component_info` (Gate/Switch/Lamp only), so the two
/// panels never show at once. **One persistent entity**, spawned once at
/// `Startup`, same reasoning as the other detail panels.
#[derive(Component)]
pub(crate) struct PinHeaderPanel;

#[derive(Component)]
pub(crate) struct PinHeaderLabelField;

#[derive(Component)]
pub(crate) struct PinHeaderLabelFieldText;

#[derive(Component)]
pub(crate) struct PinHeaderLabelSuggestions;

#[derive(Component, Clone)]
pub(crate) struct PinHeaderLabelSuggestionButton(pub String);

#[derive(Component)]
pub(crate) struct PinHeaderLinkedText;

const PIN_HEADER_LABEL_PLACEHOLDER: &str = "Label de connexion";
const PIN_HEADER_DESCRIPTION: &str = "Pin : point de connexion électrique relié au circuit intérieur — porte le même label qu'un Pin/Lampe de la vue structure pour être lié.";

fn pin_header_panel_caption(font: Handle<Font>, text: &str) -> impl Bundle {
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

fn pin_header_panel_text(font: Handle<Font>) -> impl Bundle {
    (
        TextFont {
            font: font.into(),
            font_size: LABEL_FONT_SIZE.into(),
            ..default()
        },
        TextColor(Color::WHITE),
    )
}

/// Same `Node`/color styling as `inspector::spawn_inspector_panel` /
/// `chip_structure::spawn_structure_block_panel` — bottom right, fixed.
pub fn spawn_pin_header_panel(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT_PATH);
    commands
        .spawn((
            PinHeaderPanel,
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
            parent.spawn(pin_header_panel_caption(font.clone(), "Nom"));
            parent.spawn((
                Button,
                PinHeaderLabelField,
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
                    PinHeaderLabelFieldText,
                    Text::new(PIN_HEADER_LABEL_PLACEHOLDER),
                    TextFont {
                        font: font.clone().into(),
                        font_size: LABEL_FONT_SIZE.into(),
                        ..default()
                    },
                    TextColor(COLOR_BUTTON_BORDER),
                )],
            ));
            parent.spawn((
                PinHeaderLabelSuggestions,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    ..default()
                },
            ));
            parent.spawn(pin_header_panel_caption(font.clone(), "Description"));
            parent.spawn((
                Text::new(PIN_HEADER_DESCRIPTION),
                pin_header_panel_text(font.clone()),
            ));
            parent.spawn((
                PinHeaderLinkedText,
                Text::new(""),
                pin_header_panel_text(font),
            ));
        });
}

/// Shows/hides the panel from `Selected`, filtered to entities carrying
/// `PinHeader` — same `is_changed()`-gated pattern as
/// `chip_structure::sync_structure_block_panel`. The "Lié" line checks
/// whether the selected Pin's label matches *any other*
/// `StructurePinLabel`-carrying entity, structure or interior alike — same
/// simplicity trade-off (refreshes on reselection, not live) documented on
/// the structure panel's own equivalent check.
pub fn sync_pin_header_panel(
    selected: Res<Selected>,
    pin_headers: Query<(), With<PinHeader>>,
    labels: Query<(Entity, &StructurePinLabel)>,
    mut panel: Query<&mut Node, With<PinHeaderPanel>>,
    mut linked_text: Query<&mut Text, With<PinHeaderLinkedText>>,
) {
    if !selected.is_changed() {
        return;
    }
    let Ok(mut node) = panel.single_mut() else {
        return;
    };
    let is_pin_header = selected
        .0
        .is_some_and(|entity| pin_headers.get(entity).is_ok());
    node.display = if is_pin_header {
        Display::Flex
    } else {
        Display::None
    };
    if !is_pin_header {
        return;
    }
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

/// Focuses the label field on click; defocuses on Entrée/Échap or any other
/// click — mirrors `chip_structure::handle_structure_pin_label_field_click`.
pub fn handle_pin_header_label_field_click(
    keys: Res<ButtonInput<KeyCode>>,
    pointer: Res<PointerState>,
    pointer_over_ui: Res<PointerOverUi>,
    mut focus: ResMut<PinHeaderLabelFocus>,
    field: Query<&Interaction, (Changed<Interaction>, With<PinHeaderLabelField>)>,
    other_buttons: Query<&Interaction, (Changed<Interaction>, Without<PinHeaderLabelField>)>,
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

/// Appends typed characters to the *selected* `PinHeader`'s
/// `StructurePinLabel` while the field is focused — mirrors
/// `chip_structure::handle_structure_pin_label_typing`.
pub fn handle_pin_header_label_typing(
    focus: Res<PinHeaderLabelFocus>,
    selected: Res<Selected>,
    mut keys: MessageReader<KeyboardInput>,
    pin_headers: Query<(), With<PinHeader>>,
    mut labels: Query<&mut StructurePinLabel>,
) {
    let target = focus.0.then_some(selected.0).flatten().and_then(|entity| {
        pin_headers
            .get(entity)
            .ok()
            .and_then(|()| labels.get_mut(entity).ok())
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

/// Keeps the field's displayed text in sync with the selected `PinHeader`'s
/// `StructurePinLabel` — mirrors
/// `chip_structure::sync_structure_pin_label_field_text` (unconditional,
/// same reasoning).
pub fn sync_pin_header_label_field_text(
    selected: Res<Selected>,
    pin_headers: Query<(), With<PinHeader>>,
    labels: Query<&StructurePinLabel>,
    mut text: Query<&mut Text, With<PinHeaderLabelFieldText>>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let label = selected.0.and_then(|entity| {
        pin_headers
            .get(entity)
            .ok()
            .and_then(|()| labels.get(entity).ok())
            .map(|label| label.0.clone())
    });
    text.0 = match label {
        Some(label) if !label.is_empty() => label,
        _ => PIN_HEADER_LABEL_PLACEHOLDER.to_string(),
    };
}

/// Highlights the field's border while it holds keyboard focus — same
/// convention as `chip_structure::sync_structure_pin_label_field_border`.
pub fn sync_pin_header_label_field_border(
    focus: Res<PinHeaderLabelFocus>,
    mut field: Query<&mut BorderColor, With<PinHeaderLabelField>>,
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

/// Rebuilds the suggestion list from every distinct, non-empty
/// `StructurePinLabel` *other* than the currently selected `PinHeader` —
/// same global, unfiltered-by-kind query as
/// `chip_structure::sync_structure_pin_label_suggestions`, so labels typed
/// on a structure Pin/Lamp show up here too.
pub fn sync_pin_header_label_suggestions(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    selected: Res<Selected>,
    labels: Query<(Entity, &StructurePinLabel)>,
    suggestions_root: Query<Entity, With<PinHeaderLabelSuggestions>>,
    existing_buttons: Query<Entity, With<PinHeaderLabelSuggestionButton>>,
    mut last: Local<Vec<String>>,
) {
    let mut current: Vec<String> = labels
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
                PinHeaderLabelSuggestionButton(label.clone()),
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
/// `PinHeader`'s `StructurePinLabel`, no typing needed.
pub fn handle_pin_header_label_suggestion_click(
    selected: Res<Selected>,
    pin_headers: Query<(), With<PinHeader>>,
    buttons: Query<(&Interaction, &PinHeaderLabelSuggestionButton), Changed<Interaction>>,
    mut labels: Query<&mut StructurePinLabel>,
) {
    let Some(entity) = selected.0 else {
        return;
    };
    if pin_headers.get(entity).is_err() {
        return;
    }
    let Some((_, clicked)) = buttons.iter().find(|(i, _)| **i == Interaction::Pressed) else {
        return;
    };
    let Ok(mut label) = labels.get_mut(entity) else {
        return;
    };
    label.0 = clicked.0.clone();
}
