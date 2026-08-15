use bevy::prelude::*;

use crate::constants::PREVIEW_ALPHA;
use crate::grid::{cell_to_world, world_to_cell};

use super::cursor::cursor_world_position;
use super::hud::PointerOverUi;
use super::resources::{ArmedTool, PendingRotation, ToolKind};
use super::spawn::{facing_quat, spawn_placement_preview};

/// The root of a placement-preview ghost (one entity, holding the whole
/// thing's `Transform`; despawning it takes its visual children with it).
#[derive(Component)]
pub(crate) struct PlacementPreview;

/// Marks a preview ghost's individual sprites (body, pin ghosts) so
/// `tint_placement_preview` can dim them — everything under a
/// `PlacementPreview` root, not the root itself, since the root has no
/// `Sprite` of its own.
#[derive(Component)]
pub(crate) struct PlacementPreviewTint;

/// Shows a dimmed ghost of whatever's armed (see `ArmedTool`) at the
/// cursor's snapped cell, so the player can see what they're about to place
/// and where — and in what orientation, see `PendingRotation` — before
/// clicking. Cable is excluded — it already gets its own live preview while
/// dragging (`render_cable_drag_preview`), and doesn't have a single fixed
/// footprint to ghost before that drag starts. Respawns the ghost (cheap —
/// a handful of sprites) whenever the armed tool changes, since that's the
/// only time its *shape* needs to change; every other frame it just
/// repositions/reorients the existing one.
#[allow(clippy::too_many_arguments)]
pub fn sync_placement_preview(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    armed: Res<ArmedTool>,
    rotation: Res<PendingRotation>,
    pointer_over_ui: Res<PointerOverUi>,
    window: Single<&Window>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    mut existing: Query<(Entity, &mut Transform), With<PlacementPreview>>,
    mut last_tool: Local<Option<ToolKind>>,
) {
    let (camera, camera_transform) = *camera_query;
    let cursor_world = cursor_world_position(&window, camera, camera_transform);

    let wanted_tool = if pointer_over_ui.0 {
        None
    } else {
        armed.0.filter(|tool| *tool != ToolKind::Cable)
    };

    if wanted_tool != *last_tool {
        for (entity, _) in &existing {
            commands.entity(entity).despawn();
        }
        *last_tool = wanted_tool;

        if let (Some(tool), Some(world_pos)) = (wanted_tool, cursor_world) {
            spawn_placement_preview(
                &mut commands,
                &asset_server,
                tool,
                world_to_cell(world_pos),
                rotation.0,
            );
        }
        return;
    }

    let (Some(_), Some(world_pos)) = (wanted_tool, cursor_world) else {
        return;
    };
    let cell = world_to_cell(world_pos);
    for (_, mut transform) in &mut existing {
        let z = transform.translation.z;
        transform.translation = cell_to_world(cell).extend(z);
        transform.rotation = facing_quat(rotation.0);
    }
}

/// Keeps every preview sprite dimmed even after its appearance JSON finishes
/// loading — `apply_loaded_appearances` replaces the whole `Sprite` (with an
/// opaque default color) the moment that happens, same reason
/// `sync_pin_colors`/`sync_background_grid` re-apply their tint every frame
/// rather than once at spawn time.
pub fn tint_placement_preview(mut sprites: Query<&mut Sprite, With<PlacementPreviewTint>>) {
    for mut sprite in &mut sprites {
        sprite.color = Color::srgba(1.0, 1.0, 1.0, PREVIEW_ALPHA);
    }
}
