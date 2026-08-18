use bevy::prelude::*;

use crate::constants::{
    CABLE_ENDPOINT_HIT_RADIUS, COLOR_HOVER, COLOR_SELECTION, EDIT_DRAG_THRESHOLD, GRID_CELL_SIZE,
    SELECTION_OUTLINE_MARGIN,
};
use crate::grid::{cell_to_world, world_to_cell};
use crate::simulation::components::{Cable, GateKind, GridPosition};

use super::hud::{DeleteButton, ModeToggleButton, PointerOverUi, RotateButton};
use super::placement::pick_entity_at_cell;
use super::pointer::PointerState;
use super::resources::{
    ArmedTool, EditDragState, InteractionState, Mode, PickCycleState, Selected,
};
use super::spawn::{GATE_BODY_ROW_OFFSET, facing_quat};
use super::wiring::{CableEnd, CableHit, find_cable_at};

#[allow(clippy::too_many_arguments)]
pub fn toggle_mode(
    keys: Res<ButtonInput<KeyCode>>,
    mode_button: Query<&Interaction, (Changed<Interaction>, With<ModeToggleButton>)>,
    mut mode: ResMut<Mode>,
    mut armed: ResMut<ArmedTool>,
    mut interaction: ResMut<InteractionState>,
    mut drag: ResMut<EditDragState>,
    mut cycle: ResMut<PickCycleState>,
    mut selected: ResMut<Selected>,
) {
    let button_pressed = mode_button.iter().any(|i| *i == Interaction::Pressed);
    if !keys.just_pressed(KeyCode::Tab) && !button_pressed {
        return;
    }
    *mode = match *mode {
        Mode::Interaction => Mode::Edit,
        Mode::Edit => Mode::Interaction,
    };
    armed.0 = None;
    *interaction = InteractionState::Idle;
    *drag = EditDragState::Idle;
    *cycle = PickCycleState::default();
    selected.0 = None;
}

#[allow(clippy::too_many_arguments)]
pub fn handle_edit_click_start(
    mode: Res<Mode>,
    armed: Res<ArmedTool>,
    pointer_over_ui: Res<PointerOverUi>,
    pointer: Res<PointerState>,
    mut drag: ResMut<EditDragState>,
    mut cycle: ResMut<PickCycleState>,
    mut selected: ResMut<Selected>,
    positions: Query<(Entity, &GridPosition)>,
    cables: Query<(Entity, &Cable)>,
) {
    if *mode != Mode::Edit || armed.0.is_some() || pointer_over_ui.0 || !pointer.just_pressed {
        return;
    }
    let Some(world_pos) = pointer.world_pos else {
        return;
    };

    let cell = world_to_cell(world_pos);
    let candidates: Vec<Entity> = positions
        .iter()
        .filter(|(_, position)| position.0 == cell)
        .map(|(entity, _)| entity)
        .collect();
    if let Some(entity) = pick_entity_at_cell(cell, candidates, &mut cycle) {
        selected.0 = Some(entity);
        *drag = EditDragState::Pressed {
            entity,
            start_cursor: world_pos,
            dragged: false,
        };
        return;
    }

    // Nothing with a GridPosition was under the cursor — try a cable
    // instead, which has no GridPosition of its own and gets its own
    // endpoint-vs-body hit-test.
    let Some((entity, hit)) = find_cable_at(world_pos, &cables) else {
        // Clicked empty space: drop the current selection, same as clicking
        // outside any element in a typical editor.
        selected.0 = None;
        return;
    };
    let Ok((_, cable)) = cables.get(entity) else {
        return;
    };
    selected.0 = Some(entity);
    *drag = match hit {
        CableHit::Endpoint(which) => EditDragState::CableEndpoint {
            entity,
            which,
            start_cursor: world_pos,
            dragged: false,
        },
        CableHit::Body => EditDragState::CableBody {
            entity,
            start_cursor: world_pos,
            orig_start: cable.start,
            orig_end: cable.end,
            dragged: false,
        },
    };
}

pub fn handle_edit_drag(
    mode: Res<Mode>,
    pointer: Res<PointerState>,
    mut drag: ResMut<EditDragState>,
    mut positioned: Query<(&mut GridPosition, &mut Transform)>,
    mut cables: Query<&mut Cable>,
) {
    if *mode != Mode::Edit || !pointer.pressed {
        return;
    }
    let Some(world_pos) = pointer.world_pos else {
        return;
    };

    match *drag {
        EditDragState::Idle => {}
        EditDragState::Pressed {
            entity,
            start_cursor,
            mut dragged,
        } => {
            if !dragged && start_cursor.distance(world_pos) > EDIT_DRAG_THRESHOLD {
                dragged = true;
            }
            if dragged && let Ok((mut position, mut transform)) = positioned.get_mut(entity) {
                let target_cell = world_to_cell(world_pos);
                position.0 = target_cell;
                transform.translation = cell_to_world(target_cell).extend(transform.translation.z);
            }
            *drag = EditDragState::Pressed {
                entity,
                start_cursor,
                dragged,
            };
        }
        EditDragState::CableBody {
            entity,
            start_cursor,
            orig_start,
            orig_end,
            mut dragged,
        } => {
            if !dragged && start_cursor.distance(world_pos) > EDIT_DRAG_THRESHOLD {
                dragged = true;
            }
            if dragged && let Ok(mut cable) = cables.get_mut(entity) {
                let delta = world_to_cell(world_pos) - world_to_cell(start_cursor);
                let new_start = orig_start + delta;
                let new_end = orig_end + delta;
                // Only write when the snapped position actually moved: an
                // unconditional write here would mark `Cable` changed every
                // single dragged frame (even while held over the same
                // cell), and `rebuild_cable_segments` reacts to that by
                // despawning/respawning every segment — visible as a
                // flicker while the mouse sits still mid-drag.
                if cable.start != new_start || cable.end != new_end {
                    cable.start = new_start;
                    cable.end = new_end;
                }
            }
            *drag = EditDragState::CableBody {
                entity,
                start_cursor,
                orig_start,
                orig_end,
                dragged,
            };
        }
        EditDragState::CableEndpoint {
            entity,
            which,
            start_cursor,
            mut dragged,
        } => {
            if !dragged && start_cursor.distance(world_pos) > EDIT_DRAG_THRESHOLD {
                dragged = true;
            }
            if dragged && let Ok(mut cable) = cables.get_mut(entity) {
                let target_cell = world_to_cell(world_pos);
                // Same reasoning as the `CableBody` branch above: skip the
                // write entirely when nothing actually moved, so an idle
                // held-endpoint drag doesn't spuriously retrigger
                // `rebuild_cable_segments` every frame.
                match which {
                    CableEnd::Start if cable.start != target_cell => cable.start = target_cell,
                    CableEnd::End if cable.end != target_cell => cable.end = target_cell,
                    _ => {}
                }
            }
            *drag = EditDragState::CableEndpoint {
                entity,
                which,
                start_cursor,
                dragged,
            };
        }
    }
}

/// Ends a press-drag started by `handle_edit_click_start`. Selection itself
/// already happened on press; a plain (non-dragged) release has nothing left
/// to do, and a dragged one already applied its move live in
/// `handle_edit_drag` — this just resets the drag state back to idle.
pub fn handle_edit_click_end(
    mode: Res<Mode>,
    pointer: Res<PointerState>,
    mut drag: ResMut<EditDragState>,
) {
    if *mode != Mode::Edit || !pointer.just_released {
        return;
    }
    *drag = EditDragState::Idle;
}

/// Deletes the currently-`Selected` entity on Delete/Backspace, Edit mode
/// only. Cables no longer reference `Pin` entities (connectivity is
/// spatial), so despawning here — recursively taking a component's
/// pins/label children with it, same as always — needs no extra cascade.
pub fn handle_delete_selected(
    mode: Res<Mode>,
    keys: Res<ButtonInput<KeyCode>>,
    delete_button: Query<&Interaction, (Changed<Interaction>, With<DeleteButton>)>,
    mut commands: Commands,
    mut selected: ResMut<Selected>,
) {
    let button_pressed = delete_button.iter().any(|i| *i == Interaction::Pressed);
    if *mode != Mode::Edit
        || !(keys.just_pressed(KeyCode::Delete)
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

/// Rotates the currently-`Selected` entity by one quarter turn in place: `R`
/// or the right arrow clockwise, left arrow counter-clockwise. Same keys as
/// `handle_rotation_input`'s armed-tool preview, kept from fighting over
/// them by only acting when nothing is armed (an armed tool always wins —
/// see the guard below). Works freely alongside an in-progress drag — a
/// component drag only ever touches its `Transform`'s translation, so a
/// same-frame rotation never conflicts there, and a whole-cable
/// (`CableBody`) drag gets explicitly re-anchored below so the two compose
/// instead of fighting. Handles both shapes a selection can be: a placed
/// component (own `Transform`/facing) or a cable (no facing of its own —
/// see the `cables` branch below).
#[allow(clippy::too_many_arguments)]
pub fn handle_selected_rotation(
    mode: Res<Mode>,
    armed: Res<ArmedTool>,
    keys: Res<ButtonInput<KeyCode>>,
    selected: Res<Selected>,
    pointer: Res<PointerState>,
    rotate_button: Query<&Interaction, (Changed<Interaction>, With<RotateButton>)>,
    mut drag: ResMut<EditDragState>,
    mut positioned: Query<&mut Transform, With<GridPosition>>,
    mut cables: Query<&mut Cable>,
) {
    if *mode != Mode::Edit || armed.0.is_some() {
        return;
    }
    let Some(entity) = selected.0 else {
        return;
    };
    let button_pressed = rotate_button.iter().any(|i| *i == Interaction::Pressed);
    let delta = if keys.just_pressed(KeyCode::KeyR)
        || keys.just_pressed(KeyCode::ArrowRight)
        || button_pressed
    {
        1
    } else if keys.just_pressed(KeyCode::ArrowLeft) {
        -1
    } else {
        return;
    };

    if let Ok(mut transform) = positioned.get_mut(entity) {
        // Inverse of `facing_quat`: recover the current quarter-turn count
        // from the root's Z rotation, so repeated presses step from
        // wherever the component was originally placed rather than
        // resetting to a fixed orientation.
        let current =
            (-transform.rotation.to_scaled_axis().z / std::f32::consts::FRAC_PI_2).round() as i32;
        let turns = (current + delta).rem_euclid(4) as u8;
        transform.rotation = facing_quat(turns);
        return;
    }

    if let Ok(mut cable) = cables.get_mut(entity) {
        // A cable has no facing of its own to spin — instead, pivot both
        // endpoints one quarter turn around a point near its own center;
        // see `rotate_cable_endpoints` for why that pivot doesn't drift.
        let (new_start, new_end) = rotate_cable_endpoints(cable.start, cable.end, delta > 0);
        cable.start = new_start;
        cable.end = new_end;

        // `handle_edit_drag`'s `CableBody` branch recomputes `start`/`end`
        // every dragged frame as `orig + delta` measured from the press
        // point — left alone, the very next frame would snap right back to
        // the pre-rotation shape. Re-anchoring both the reference shape and
        // the reference cursor position to right now makes that recompute a
        // no-op until the mouse actually moves again, so the rotation
        // sticks and dragging continues smoothly from the new orientation.
        if let EditDragState::CableBody {
            entity: dragged_entity,
            dragged,
            ..
        } = *drag
            && dragged_entity == entity
            && let Some(world_pos) = pointer.world_pos
        {
            *drag = EditDragState::CableBody {
                entity,
                start_cursor: world_pos,
                orig_start: new_start,
                orig_end: new_end,
                dragged,
            };
        }
    }
}

/// Pivots a cable's two endpoints one quarter turn (`clockwise` or not)
/// around a point near the cable's own center, always landing back on exact
/// grid nodes (connectivity is spatial, see `wiring.rs`, so an off-grid
/// endpoint would silently stop connecting to anything).
///
/// The pivot is `start + trunc(arm / 2)` (`arm = end - start`) rather than
/// the more obvious `round((start + end) / 2)` — both give the exact true
/// center when `arm` is even on an axis, but for an odd arm the
/// rounded-average version picks a *different* grid node after each
/// rotation (its rounding direction depends on the current absolute
/// coordinates, which change every turn), so the whole cable visibly
/// creeps across the grid over repeated rotations instead of turning in
/// place. `start + trunc(arm / 2)` doesn't drift: Rust's integer division
/// truncates toward zero, so it commutes with the swap-and-negate rotation
/// below (`trunc(rotate(v) / 2) == rotate(trunc(v / 2))`), which makes the
/// computed pivot provably identical before and after every single
/// rotation — the cable turns in place, always returning to its exact
/// original position after four quarter-turns, at the cost of sitting up
/// to half a cell off the true geometric center on an odd span (barely
/// noticeable, and stable beats perfectly centered).
fn rotate_cable_endpoints(start: IVec2, end: IVec2, clockwise: bool) -> (IVec2, IVec2) {
    let arm = end - start;
    let center = start + IVec2::new(arm.x / 2, arm.y / 2);
    let rotate = |v: IVec2| {
        if clockwise {
            IVec2::new(v.y, -v.x)
        } else {
            IVec2::new(-v.y, v.x)
        }
    };
    (
        center + rotate(start - center),
        center + rotate(end - center),
    )
}

/// Draws an outline around one entity — a rotated box matching a placed
/// component's facing and footprint (2 cells tall for AND/OR gates, see
/// `GATE_BODY_ROW_OFFSET`; 1 cell otherwise), or a highlighted line +
/// endpoint markers for a cable. Shared by `render_selection_highlight`
/// (magenta) and `render_hover_highlight` (dimmer cyan) so the two stay
/// visually consistent.
fn draw_entity_outline(
    gizmos: &mut Gizmos,
    entity: Entity,
    positioned: &Query<(&Transform, Option<&GateKind>), With<GridPosition>>,
    cables: &Query<(Entity, &Cable)>,
    color: Color,
) {
    if let Ok((transform, gate_kind)) = positioned.get(entity) {
        let (local_offset, size) = match gate_kind {
            Some(GateKind::And | GateKind::Or) => (
                Vec2::new(0.0, GATE_BODY_ROW_OFFSET),
                Vec2::new(GRID_CELL_SIZE, GRID_CELL_SIZE * 2.0),
            ),
            _ => (Vec2::ZERO, Vec2::splat(GRID_CELL_SIZE)),
        };
        let rotation = Rot2::radians(transform.rotation.to_scaled_axis().z);
        let center = transform.translation.truncate() + rotation * local_offset;
        gizmos.rect_2d(
            Isometry2d::new(center, rotation),
            size + Vec2::splat(SELECTION_OUTLINE_MARGIN),
            color,
        );
        return;
    }

    if let Ok((_, cable)) = cables.get(entity) {
        let start = cell_to_world(cable.start);
        let end = cell_to_world(cable.end);
        gizmos.line_2d(start, end, color);
        gizmos.circle_2d(start, CABLE_ENDPOINT_HIT_RADIUS * 0.6, color);
        gizmos.circle_2d(end, CABLE_ENDPOINT_HIT_RADIUS * 0.6, color);
    }
}

/// Draws a magenta outline around the `Selected` entity in Edit mode — see
/// `draw_entity_outline`.
pub fn render_selection_highlight(
    mode: Res<Mode>,
    selected: Res<Selected>,
    positioned: Query<(&Transform, Option<&GateKind>), With<GridPosition>>,
    cables: Query<(Entity, &Cable)>,
    mut gizmos: Gizmos,
) {
    if *mode != Mode::Edit {
        return;
    }
    let Some(entity) = selected.0 else {
        return;
    };
    draw_entity_outline(&mut gizmos, entity, &positioned, &cables, COLOR_SELECTION);
}

/// Highlights whatever the cursor is over in Edit mode (dimmer cyan, see
/// `COLOR_HOVER`) before the player commits to a click — same hit-test as
/// `handle_edit_click_start` (first a `GridPosition` match at the cell, then
/// a cable body/endpoint), but read-only and re-evaluated every frame
/// instead of only on press. Suppressed while a tool is armed (the
/// placement-preview ghost owns that case instead), the pointer is over UI,
/// or an edit drag is already in progress (nothing new to preview
/// mid-drag). Skips drawing when the hovered entity is already the
/// selection, so the two outlines don't stack.
#[allow(clippy::too_many_arguments)]
pub fn render_hover_highlight(
    mode: Res<Mode>,
    armed: Res<ArmedTool>,
    pointer_over_ui: Res<PointerOverUi>,
    drag: Res<EditDragState>,
    selected: Res<Selected>,
    pointer: Res<PointerState>,
    grid_positions: Query<(Entity, &GridPosition)>,
    positioned: Query<(&Transform, Option<&GateKind>), With<GridPosition>>,
    cables: Query<(Entity, &Cable)>,
    mut gizmos: Gizmos,
) {
    if *mode != Mode::Edit
        || armed.0.is_some()
        || pointer_over_ui.0
        || !matches!(*drag, EditDragState::Idle)
    {
        return;
    }
    let Some(world_pos) = pointer.world_pos else {
        return;
    };
    let cell = world_to_cell(world_pos);

    let hovered = grid_positions
        .iter()
        .find(|(_, position)| position.0 == cell)
        .map(|(entity, _)| entity)
        .or_else(|| find_cable_at(world_pos, &cables).map(|(entity, _)| entity));

    let Some(entity) = hovered else {
        return;
    };
    if Some(entity) == selected.0 {
        return;
    }
    draw_entity_outline(&mut gizmos, entity, &positioned, &cables, COLOR_HOVER);
}

#[cfg(test)]
mod rotate_cable_endpoints_tests {
    use super::*;

    /// Four quarter-turns in the same direction must land exactly back on
    /// the starting endpoints, for every parity of span this project
    /// actually places cables between (even/even, odd/odd axis-aligned, and
    /// mixed-parity diagonal) — the regression this guards against is the
    /// old rounded-average pivot silently drifting the cable across the
    /// grid over repeated rotations instead of turning it in place.
    fn assert_four_turns_return_to_start(start: IVec2, end: IVec2) {
        let mut current = (start, end);
        for _ in 0..4 {
            current = rotate_cable_endpoints(current.0, current.1, true);
        }
        assert_eq!(current, (start, end));

        let mut current = (start, end);
        for _ in 0..4 {
            current = rotate_cable_endpoints(current.0, current.1, false);
        }
        assert_eq!(current, (start, end));
    }

    #[test]
    fn four_turns_return_to_start_even_length_horizontal() {
        assert_four_turns_return_to_start(IVec2::new(0, 0), IVec2::new(2, 0));
    }

    #[test]
    fn four_turns_return_to_start_odd_length_horizontal() {
        assert_four_turns_return_to_start(IVec2::new(0, 0), IVec2::new(3, 0));
    }

    #[test]
    fn four_turns_return_to_start_odd_length_negative_coords() {
        assert_four_turns_return_to_start(IVec2::new(-4, 7), IVec2::new(-1, 7));
    }

    #[test]
    fn four_turns_return_to_start_mixed_parity_diagonal() {
        assert_four_turns_return_to_start(IVec2::new(0, 0), IVec2::new(3, 2));
    }

    #[test]
    fn single_turn_pivots_at_true_center_when_span_is_even() {
        let (new_start, new_end) = rotate_cable_endpoints(IVec2::new(0, 0), IVec2::new(2, 0), true);
        assert_eq!(new_start, IVec2::new(1, 1));
        assert_eq!(new_end, IVec2::new(1, -1));
    }

    #[test]
    fn rotation_preserves_span_length() {
        let (start, end) = (IVec2::new(-2, 5), IVec2::new(3, 1));
        let original_len_sq = (end - start).length_squared();
        let (new_start, new_end) = rotate_cable_endpoints(start, end, true);
        assert_eq!((new_end - new_start).length_squared(), original_len_sq);
    }
}
