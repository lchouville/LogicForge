use std::collections::HashMap;

use bevy::prelude::*;

use crate::simulation::components::{Cable, GateKind, GridPosition, Lamp, PinHeader, Switch};

use super::camera_control::{CameraPanState, PinchState};
use super::chip_instance::{ChipInstance, spawn_chip_instance};
use super::chip_structure::{
    ActiveStructureColor, ActiveStructureLabel, SelectedStructureBlock, StructureBlockKind,
    StructureCell, StructureDragState, StructurePinLabel, StructurePinLabelFocus,
    spawn_structure_block,
};
use super::chip_view::PreChipEditCamera;
use super::edit_mode::reset_transient_editor_state;
use super::resources::TransientEditorState;
use super::spawn::{
    rotation_from_transform, spawn_and_or_gate, spawn_lamp, spawn_not_gate, spawn_pin_header,
    spawn_switch,
};
use crate::constants::SPAWN_Z_STEP;
use crate::rendering::cable::spawn_cable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectId(pub u32);

pub struct ProjectEntry {
    pub id: ProjectId,
    pub name: String,
}

/// A chip's display name, its Corps tint, and its block layout — see
/// `ProjectLibrary::chip_blueprint`.
pub type ChipBlueprint = (String, Color, Vec<(IVec2, StructureBlockKind, String)>);

/// A placed circuit entity, captured just enough to respawn it identically
/// via the same `spawn_*` functions used for live placement — see
/// `switch_to_project`. Deliberately not `#[derive(Serialize)]`: this never
/// leaves memory, so plain Rust data is enough (every field here is already
/// `Copy` on its source component).
enum SavedEntity {
    AndOrGate {
        kind: GateKind,
        cell: IVec2,
        rotation: u8,
    },
    NotGate {
        cell: IVec2,
        rotation: u8,
    },
    Switch {
        cell: IVec2,
        rotation: u8,
        on: bool,
    },
    Lamp {
        cell: IVec2,
        rotation: u8,
    },
    Pin {
        cell: IVec2,
        rotation: u8,
        label: String,
    },
    Chip {
        cell: IVec2,
        rotation: u8,
        source: ProjectId,
        display_name: String,
        body_color: Color,
        blocks: Vec<(IVec2, StructureBlockKind, String)>,
    },
    Cable {
        start: IVec2,
        end: IVec2,
    },
}

struct SavedCamera {
    translation: Vec2,
    scale: f32,
}

/// A structure block, captured just enough to respawn it via
/// `chip_structure::spawn_structure_block` — same reasoning as
/// `SavedEntity`. `label` is only ever non-empty for a `Pin` or `Lamp` block
/// (Corps has no `StructurePinLabel` component to read).
struct SavedStructureBlock {
    kind: StructureBlockKind,
    cell: IVec2,
    label: String,
}

/// Matches every root circuit entity (gate/switch/lamp all carry
/// `GridPosition`, a cable carries `Cable`) — used both to bulk-despawn the
/// outgoing project's circuit and, in `sidebar::handle_project_selection`, to
/// prove that query disjoint from the camera's own `Transform` access.
pub type CircuitEntityFilter = Or<(With<GridPosition>, With<Cable>)>;

/// A project's circuit while it isn't the active one — `None` camera means
/// "never visited yet, use the default view". `structure_color`/
/// `structure_label: None` mean "never customized yet", so a fresh project
/// starts on `ActiveStructureColor`/`ActiveStructureLabel`'s own defaults
/// rather than baking those defaults in here too.
#[derive(Default)]
struct ProjectData {
    entities: Vec<SavedEntity>,
    camera: Option<SavedCamera>,
    structure_entities: Vec<SavedStructureBlock>,
    structure_color: Option<Color>,
    structure_label: Option<String>,
}

/// The full set of projects (flat list for now — no sub-folders yet, see the
/// roadmap plan) plus the parked circuit data for every project that isn't
/// currently live in the world. Only the active project's entities actually
/// exist as ECS entities at any given time; every other project's circuit is
/// held here as data until switched back to.
#[derive(Resource)]
pub struct ProjectLibrary {
    pub entries: Vec<ProjectEntry>,
    data: HashMap<ProjectId, ProjectData>,
    pub active: ProjectId,
    next_id: u32,
}

impl ProjectLibrary {
    fn allocate_id(&mut self) -> ProjectId {
        let id = ProjectId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Adds a new empty project to the list and returns its id — does not
    /// switch to it; callers that want that (e.g. "Nouveau projet") should
    /// follow up with `switch_to_project`, which already treats a missing
    /// `data` entry as an empty project.
    pub fn create_project(&mut self) -> ProjectId {
        let id = self.allocate_id();
        self.entries.push(ProjectEntry {
            id,
            // `id.0` alone (not `+ 1`) reads as "Projet 1" for the first
            // created project: id 0 is reserved for `FREE_MODE_PROJECT` and
            // never allocated here (`next_id` starts at 1).
            name: format!("Projet {}", id.0),
        });
        id
    }

    /// A frozen snapshot of `id`'s structure (display name, Corps tint,
    /// block layout) for `chip_instance::spawn_chip_instance` to place a
    /// copy of it as a component elsewhere — `None` if `id` was never
    /// visited (no saved data) or has no structure blocks at all (nothing
    /// to place), both treated identically by callers as "nothing to
    /// place". Deliberately returns owned data rather than borrows: the
    /// caller (placement) immediately bakes this into a `ChipInstance` that
    /// outlives this project's own data, so there's nothing to keep this
    /// borrowed against.
    pub fn chip_blueprint(&self, id: ProjectId) -> Option<ChipBlueprint> {
        let data = self.data.get(&id)?;
        if data.structure_entities.is_empty() {
            return None;
        }
        let display_name = data
            .structure_label
            .as_ref()
            .filter(|label| !label.is_empty())
            .cloned()
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|entry| entry.id == id)
                    .map(|entry| entry.name.clone())
            })
            .unwrap_or_default();
        let body_color = data
            .structure_color
            .unwrap_or(ActiveStructureColor::default().0);
        let blocks = data
            .structure_entities
            .iter()
            .map(|block| (block.cell, block.kind, block.label.clone()))
            .collect();
        Some((display_name, body_color, blocks))
    }
}

/// Which of a project's two screens is showing: the normal circuit editor,
/// or the chip structure editor (Corps/Pin/Lampe blocks — see
/// `chip_structure.rs`). Reset to `Standard` on every project switch rather
/// than remembered per-project, to keep v1 simple.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub enum ProjectView {
    #[default]
    Standard,
    ChipEdit,
}

/// Bundles `ProjectView` + `PreChipEditCamera` + the structure editor's own
/// selection/drag state for `switch_to_project` — a project switch always
/// forces the view back to `Standard` and clears every piece of transient
/// state that could reference a structure block about to be despawned (same
/// reasoning as `TransientEditorState` for the interior circuit). Spelling
/// these out as separate parameters on `sidebar::handle_project_selection`
/// (already at 15) would push it past Bevy's 16-parameter system limit —
/// same reasoning as `resources::TransientEditorState` itself.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ViewSwitchState<'w> {
    pub view: ResMut<'w, ProjectView>,
    pub pre_chip_edit_camera: ResMut<'w, PreChipEditCamera>,
    pub selected_structure: ResMut<'w, SelectedStructureBlock>,
    pub structure_drag: ResMut<'w, StructureDragState>,
    /// Shared between `handle_camera_pan`/`handle_structure_camera_pan` —
    /// forcing the view to `Standard` here can hand off a pan/pinch left
    /// mid-gesture to the interior editor's own pan system, which would
    /// reproject its stale screen position through a suddenly very
    /// different camera transform (same hazard `handle_chip_view_toggle_click`
    /// already guards against for the plain view-toggle case).
    pub camera_pan: ResMut<'w, CameraPanState>,
    pub pinch: ResMut<'w, PinchState>,
    /// A focused per-pin label field is stale the instant its target Pin's
    /// project is no longer active — same reasoning as `selected_structure`
    /// above.
    pub pin_label_focus: ResMut<'w, StructurePinLabelFocus>,
}

/// Bundles the two per-project structure customization resources
/// (`ActiveStructureColor`, `ActiveStructureLabel`) into one system-param
/// slot for `switch_to_project`'s callers — same reasoning as
/// `ViewSwitchState` above: `sidebar::handle_project_selection` is already
/// at Bevy's 16-parameter system limit, so a second bare `ResMut` here would
/// push it over.
#[derive(bevy::ecs::system::SystemParam)]
pub struct StructureCustomization<'w> {
    pub color: ResMut<'w, ActiveStructureColor>,
    pub label: ResMut<'w, ActiveStructureLabel>,
}

/// Bundles every interior-circuit entity query `switch_to_project` needs to
/// snapshot into one system-param slot for its callers — same reasoning as
/// `ViewSwitchState`/`StructureCustomization` above: adding the `PinHeader`
/// query as a 5th bare `Query` on `sidebar::handle_project_selection`
/// (already at Bevy's 16-parameter system limit) would push it over.
#[derive(bevy::ecs::system::SystemParam)]
pub struct CircuitQueries<'w, 's> {
    pub gates: Query<'w, 's, (&'static GateKind, &'static GridPosition, &'static Transform)>,
    pub switches: Query<'w, 's, (&'static Switch, &'static GridPosition, &'static Transform)>,
    pub lamps: Query<'w, 's, (&'static Lamp, &'static GridPosition, &'static Transform)>,
    pub pin_headers: Query<
        'w,
        's,
        (
            &'static GridPosition,
            &'static Transform,
            Option<&'static StructurePinLabel>,
        ),
        With<PinHeader>,
    >,
    pub chip_instances: Query<
        'w,
        's,
        (
            &'static ChipInstance,
            &'static GridPosition,
            &'static Transform,
        ),
    >,
    pub cables: Query<'w, 's, &'static Cable>,
}

/// Starts the player on a real, visible, selected "Projet 1" — same
/// `create_project` auto-naming as any project the player creates later, no
/// special-cased hidden entry (an earlier version of this reserved id `0`
/// for an unlisted "free mode" project; the player found that confusing —
/// see `notes/claude/2026-08-19.md` — so it's gone).
pub fn init_project_library(mut commands: Commands) {
    let mut library = ProjectLibrary {
        entries: Vec::new(),
        data: HashMap::new(),
        active: ProjectId(0),
        next_id: 1,
    };
    library.active = library.create_project();
    commands.insert_resource(library);
}

/// Snapshots the currently-active circuit into `library`'s data for its own
/// id, despawns it from the world, then respawns `target`'s saved circuit
/// (or nothing, if `target` has never been visited) and restores its camera
/// view. No-op if `target` is already active. Also resets every piece of
/// transient editor state that could reference a now-despawned entity —
/// same reasoning as `reset_transient_editor_state`'s own doc comment.
#[allow(clippy::too_many_arguments)]
pub fn switch_to_project(
    target: ProjectId,
    library: &mut ProjectLibrary,
    commands: &mut Commands,
    asset_server: &AssetServer,
    circuit: &CircuitQueries,
    despawn_targets: &Query<Entity, CircuitEntityFilter>,
    structure_blocks: &Query<(
        &StructureBlockKind,
        &StructureCell,
        Option<&StructurePinLabel>,
    )>,
    structure_despawn_targets: &Query<Entity, With<StructureCell>>,
    customization: &mut StructureCustomization,
    camera_transform: &mut Transform,
    projection: &mut Projection,
    state: &mut TransientEditorState,
    view_switch: &mut ViewSwitchState,
) {
    if target == library.active {
        return;
    }

    let mut outgoing = ProjectData::default();
    for (kind, position, transform) in circuit.gates.iter() {
        let rotation = rotation_from_transform(transform);
        outgoing.entities.push(match kind {
            GateKind::And | GateKind::Or => SavedEntity::AndOrGate {
                kind: *kind,
                cell: position.0,
                rotation,
            },
            GateKind::Not => SavedEntity::NotGate {
                cell: position.0,
                rotation,
            },
        });
    }
    for (switch, position, transform) in circuit.switches.iter() {
        outgoing.entities.push(SavedEntity::Switch {
            cell: position.0,
            rotation: rotation_from_transform(transform),
            on: switch.on,
        });
    }
    for (_, position, transform) in circuit.lamps.iter() {
        outgoing.entities.push(SavedEntity::Lamp {
            cell: position.0,
            rotation: rotation_from_transform(transform),
        });
    }
    for (position, transform, label) in circuit.pin_headers.iter() {
        outgoing.entities.push(SavedEntity::Pin {
            cell: position.0,
            rotation: rotation_from_transform(transform),
            label: label.map(|l| l.0.clone()).unwrap_or_default(),
        });
    }
    for (instance, position, transform) in circuit.chip_instances.iter() {
        outgoing.entities.push(SavedEntity::Chip {
            cell: position.0,
            rotation: rotation_from_transform(transform),
            source: instance.source,
            display_name: instance.display_name.clone(),
            body_color: instance.body_color,
            blocks: instance.blocks.clone(),
        });
    }
    for cable in circuit.cables.iter() {
        outgoing.entities.push(SavedEntity::Cable {
            start: cable.start,
            end: cable.end,
        });
    }
    outgoing.camera = Some(SavedCamera {
        translation: camera_transform.translation.truncate(),
        scale: orthographic_scale(projection),
    });
    for (kind, cell, label) in structure_blocks.iter() {
        outgoing.structure_entities.push(SavedStructureBlock {
            kind: *kind,
            cell: cell.0,
            label: label.map(|l| l.0.clone()).unwrap_or_default(),
        });
    }
    outgoing.structure_color = Some(customization.color.0);
    outgoing.structure_label = Some(customization.label.0.clone());
    library.data.insert(library.active, outgoing);

    for entity in despawn_targets.iter() {
        commands.entity(entity).despawn();
    }
    for entity in structure_despawn_targets.iter() {
        commands.entity(entity).despawn();
    }

    state.spawn_order.0 = 0.0;
    let incoming = library.data.remove(&target).unwrap_or_default();
    for saved in &incoming.entities {
        let z = state.spawn_order.0;
        state.spawn_order.0 += SPAWN_Z_STEP;
        match *saved {
            SavedEntity::AndOrGate {
                kind,
                cell,
                rotation,
            } => {
                spawn_and_or_gate(commands, asset_server, cell, kind, rotation, z);
            }
            SavedEntity::NotGate { cell, rotation } => {
                spawn_not_gate(commands, asset_server, cell, rotation, z);
            }
            SavedEntity::Switch { cell, rotation, on } => {
                let entity = spawn_switch(commands, asset_server, cell, rotation, z);
                if on {
                    commands.entity(entity).insert(Switch { on: true });
                }
            }
            SavedEntity::Lamp { cell, rotation } => {
                spawn_lamp(commands, asset_server, cell, rotation, z);
            }
            SavedEntity::Pin {
                cell,
                rotation,
                ref label,
            } => {
                spawn_pin_header(commands, asset_server, cell, rotation, label, z);
            }
            SavedEntity::Chip {
                cell,
                rotation,
                source,
                ref display_name,
                body_color,
                ref blocks,
            } => {
                spawn_chip_instance(
                    commands,
                    asset_server,
                    cell,
                    rotation,
                    ChipInstance {
                        source,
                        display_name: display_name.clone(),
                        body_color,
                        blocks: blocks.clone(),
                    },
                    z,
                );
            }
            SavedEntity::Cable { start, end } => {
                spawn_cable(commands, asset_server, start, end);
            }
        }
    }
    customization.color.0 = incoming
        .structure_color
        .unwrap_or(ActiveStructureColor::default().0);
    customization.label.0 = incoming.structure_label.unwrap_or_default();
    for saved in &incoming.structure_entities {
        let z = state.spawn_order.0;
        state.spawn_order.0 += SPAWN_Z_STEP;
        spawn_structure_block(
            commands,
            asset_server,
            saved.cell,
            saved.kind,
            customization.color.0,
            &saved.label,
            z,
        );
    }

    let (new_translation, new_scale) = match incoming.camera {
        Some(saved) => (saved.translation, saved.scale),
        None => (Vec2::ZERO, 1.0),
    };
    camera_transform.translation = new_translation.extend(camera_transform.translation.z);
    if let Projection::Orthographic(ortho) = projection {
        ortho.scale = new_scale;
    }

    library.active = target;

    reset_transient_editor_state(
        &mut state.armed,
        &mut state.interaction,
        &mut state.drag,
        &mut state.cycle,
        &mut state.selected,
        &mut state.pin_header_label_focus,
    );

    // A project switch always lands in the interior circuit's Standard view
    // — `camera_transform` above was just set to *that* project's saved
    // interior view, so a stale `ChipEdit` here would show it through the
    // structure editor's toolbar/input instead, and clicking would try to
    // place structure blocks at nonsense coordinates (interior position
    // minus `STRUCTURE_SPACE_OFFSET`).
    *view_switch.view = ProjectView::Standard;
    view_switch.pre_chip_edit_camera.clear();
    view_switch.selected_structure.0 = None;
    *view_switch.structure_drag = StructureDragState::Idle;
    *view_switch.camera_pan = CameraPanState::Idle;
    *view_switch.pinch = PinchState::default();
    view_switch.pin_label_focus.0 = false;
}

fn orthographic_scale(projection: &Projection) -> f32 {
    match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    }
}
