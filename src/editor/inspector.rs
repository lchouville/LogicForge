use bevy::prelude::*;

use crate::constants::{COLOR_BUTTON_BORDER, COLOR_BUTTON_NORMAL, LABEL_FONT_SIZE, UI_FONT_PATH};
use crate::simulation::components::{GateKind, Lamp, Switch};

use super::resources::{Mode, Selected};

/// Fixed placeholder metadata shown for every native (built-in) component —
/// stands in for the real per-chip creator/creation-date that custom chips
/// (roadmap item 5) will eventually carry once players can author their own.
const NATIVE_CREATOR: &str = "LogicForge";
const NATIVE_CREATION_DATE: &str = "2026";

#[derive(Component)]
pub struct InspectorPanel;

#[derive(Component)]
pub struct InspectorName;

#[derive(Component)]
pub struct InspectorDescription;

#[derive(Component)]
pub struct InspectorCreator;

#[derive(Component)]
pub struct InspectorDate;

/// Spawns the (initially hidden) selected-component detail panel — see
/// `sync_inspector_panel`, which shows/hides and fills it in. Anchored
/// bottom-right so it doesn't compete with the mode label (top-left) or the
/// toolbar (bottom-left).
pub fn spawn_inspector_panel(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load(UI_FONT_PATH);
    commands.spawn((
        InspectorPanel,
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
        children![
            (
                InspectorName,
                Text::new(""),
                inspector_text_font(font.clone())
            ),
            (
                InspectorDescription,
                Text::new(""),
                inspector_text_font(font.clone())
            ),
            (
                InspectorCreator,
                Text::new(""),
                inspector_text_font(font.clone())
            ),
            (
                InspectorDate,
                Text::new(""),
                inspector_text_font(font.clone())
            ),
            // Placeholder for the future chip circuit exploration view
            // (roadmap item 5+, nothing to explore for native components) —
            // deliberately not a `Button`/`Interaction` target, so it reads
            // as disabled rather than clickable. Plain text, not a
            // magnifying-glass icon: `FiraMono-Medium` only covers Latin —
            // pictographic Unicode (🔍 U+1F50D) rendered as an empty
            // missing-glyph box, confirmed visually in a browser build.
            (
                Node {
                    width: Val::Px(64.0),
                    height: Val::Px(32.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(COLOR_BUTTON_NORMAL.with_alpha(0.5)),
                BorderColor::all(COLOR_BUTTON_BORDER.with_alpha(0.5)),
                children![(
                    Text::new("Loupe"),
                    TextFont {
                        font: font.into(),
                        font_size: LABEL_FONT_SIZE.into(),
                        ..default()
                    },
                    TextColor(COLOR_BUTTON_BORDER.with_alpha(0.5)),
                )],
            ),
        ],
    ));
}

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

/// Name + description shown for the selected component, or `None` for
/// anything that isn't a placeable component (e.g. a selected `Cable`, which
/// has no such identity to show).
fn selected_component_info(
    entity: Entity,
    gates: &Query<&GateKind>,
    switches: &Query<&Switch>,
    lamps: &Query<&Lamp>,
) -> Option<(&'static str, &'static str)> {
    if let Ok(kind) = gates.get(entity) {
        return Some(match kind {
            GateKind::And => (
                "AND",
                "Porte logique ET : sort à 1 seulement si toutes ses entrées sont à 1.",
            ),
            GateKind::Or => (
                "OR",
                "Porte logique OU : sort à 1 si au moins une entrée est à 1.",
            ),
            GateKind::Not => (
                "NOT",
                "Porte logique NON : inverse le signal reçu en entrée.",
            ),
        });
    }
    if switches.get(entity).is_ok() {
        return Some((
            "Switch",
            "Interrupteur : composant d'entrée activable/désactivable par le joueur.",
        ));
    }
    if lamps.get(entity).is_ok() {
        return Some((
            "Lamp",
            "Lampe : composant de sortie qui s'allume selon le signal reçu.",
        ));
    }
    None
}

type NameTextFilter = (
    With<InspectorName>,
    Without<InspectorDescription>,
    Without<InspectorCreator>,
    Without<InspectorDate>,
);
type DescriptionTextFilter = (
    With<InspectorDescription>,
    Without<InspectorName>,
    Without<InspectorCreator>,
    Without<InspectorDate>,
);
type CreatorTextFilter = (
    With<InspectorCreator>,
    Without<InspectorName>,
    Without<InspectorDescription>,
    Without<InspectorDate>,
);
type DateTextFilter = (
    With<InspectorDate>,
    Without<InspectorName>,
    Without<InspectorDescription>,
    Without<InspectorCreator>,
);

/// Shows/hides and fills in the panel spawned by `spawn_inspector_panel`
/// from the current `Selected` entity. Selection (and this panel with it)
/// only ever exists in Edit mode — see `Selected`'s own doc comment — so
/// there's nothing to show outside it.
#[allow(clippy::too_many_arguments)]
pub fn sync_inspector_panel(
    mode: Res<Mode>,
    selected: Res<Selected>,
    gates: Query<&GateKind>,
    switches: Query<&Switch>,
    lamps: Query<&Lamp>,
    mut panel: Query<&mut Node, With<InspectorPanel>>,
    mut name_text: Query<&mut Text, NameTextFilter>,
    mut description_text: Query<&mut Text, DescriptionTextFilter>,
    mut creator_text: Query<&mut Text, CreatorTextFilter>,
    mut date_text: Query<&mut Text, DateTextFilter>,
) {
    if !mode.is_changed() && !selected.is_changed() {
        return;
    }
    let Ok(mut node) = panel.single_mut() else {
        return;
    };
    let info = (*mode == Mode::Edit)
        .then_some(selected.0)
        .flatten()
        .and_then(|entity| selected_component_info(entity, &gates, &switches, &lamps));

    let Some((name, description)) = info else {
        node.display = Display::None;
        return;
    };
    node.display = Display::Flex;
    if let Ok(mut text) = name_text.single_mut() {
        text.0 = format!("Nom : {name}");
    }
    if let Ok(mut text) = description_text.single_mut() {
        text.0 = format!("Description : {description}");
    }
    if let Ok(mut text) = creator_text.single_mut() {
        text.0 = format!("Créateur : {NATIVE_CREATOR}");
    }
    if let Ok(mut text) = date_text.single_mut() {
        text.0 = format!("Créé le : {NATIVE_CREATION_DATE}");
    }
}
