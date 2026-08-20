use bevy::prelude::*;

use crate::constants::STRUCTURE_SPACE_OFFSET;

use super::chip_structure::{SelectedStructureBlock, StructureDragState};
use super::hud::ChipViewToggleButton;
use super::project::ProjectView;

/// The interior circuit's camera view, saved the moment the player enters
/// the chip structure editor so it can be restored exactly on the way back
/// — the structure editor always opens at a fixed default view instead (see
/// `handle_chip_view_toggle_click`), since a chip's exterior is small enough
/// that it never needs its own pan/zoom state.
#[derive(Resource, Default)]
pub struct PreChipEditCamera(Option<(Vec2, f32)>);

impl PreChipEditCamera {
    /// Drops any saved interior view — used by `project::switch_to_project`
    /// when forcing the view back to `Standard` on a project switch, so a
    /// later re-entry into the structure editor from the *new* project can't
    /// accidentally warp its camera back to the *previous* project's saved
    /// position.
    pub(crate) fn clear(&mut self) {
        self.0 = None;
    }
}

/// Flips `ProjectView` and jumps the camera between the interior circuit's
/// own view and the chip structure editor's fixed view at
/// `STRUCTURE_SPACE_OFFSET` — see `PreChipEditCamera`'s doc comment. The
/// same button re-fires this on the way back (`ProjectView::ChipEdit` ->
/// `Standard`), so there's no separate "Retour" button to maintain.
#[allow(clippy::too_many_arguments)]
pub fn handle_chip_view_toggle_click(
    mut view: ResMut<ProjectView>,
    mut pre_chip_edit_camera: ResMut<PreChipEditCamera>,
    mut selected_structure: ResMut<SelectedStructureBlock>,
    mut structure_drag: ResMut<StructureDragState>,
    toggle_button: Query<&Interaction, (Changed<Interaction>, With<ChipViewToggleButton>)>,
    mut camera: Single<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    if !toggle_button
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        return;
    }

    // Whichever direction the toggle goes, any structure-editor selection
    // or in-progress drag is now stale (its target may no longer be shown,
    // or the gesture can no longer be completed) — see `SelectedStructureBlock`.
    selected_structure.0 = None;
    *structure_drag = StructureDragState::Idle;

    let (transform, projection) = &mut *camera;
    let Projection::Orthographic(ortho) = projection.as_mut() else {
        return;
    };

    *view = match *view {
        ProjectView::Standard => {
            pre_chip_edit_camera.0 = Some((transform.translation.truncate(), ortho.scale));
            transform.translation = STRUCTURE_SPACE_OFFSET.extend(transform.translation.z);
            ortho.scale = 1.0;
            ProjectView::ChipEdit
        }
        ProjectView::ChipEdit => {
            let (translation, scale) = pre_chip_edit_camera.0.unwrap_or((Vec2::ZERO, 1.0));
            transform.translation = translation.extend(transform.translation.z);
            ortho.scale = scale;
            ProjectView::Standard
        }
    };
}
