use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use crate::constants::{
    CAMERA_WHEEL_PIXELS_PER_LINE, CAMERA_WHEEL_ZOOM_SENSITIVITY, CAMERA_ZOOM_MAX_SCALE,
    CAMERA_ZOOM_MIN_SCALE, EDIT_DRAG_THRESHOLD,
};
use crate::grid::world_to_cell;
use crate::simulation::components::{Cable, GridPosition};

use super::chip_instance::ChipBridgeCable;
use super::hud::PointerOverUi;
use super::pointer::PointerState;
use super::resources::ArmedTool;
use super::wiring::find_cable_at;

/// Which input started the current pan, so its own release (and only its
/// own) ends it — a middle-mouse pan shouldn't stop just because a touch
/// happens to lift, and vice versa.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PanSource {
    Middle,
    Pointer,
}

/// Camera pan state, driven by either the middle mouse button (unambiguous,
/// no click/drag distinction needed) or the primary pointer — mouse-left or
/// the one tracked touch, see `PointerState` — starting from empty space
/// (own click/drag threshold, since a plain click there still needs to
/// deselect in Edit mode). Same shape as `EditDragState`: `Panning` is
/// re-anchored every frame rather than storing a fixed start, so the camera
/// stops the instant the input stops moving instead of extrapolating.
///
/// `Panning` stores a *screen* position, not a world one, and deliberately
/// so: `PointerState::world_pos` for two different frames is computed
/// against two different camera transforms (this frame's, and whatever was
/// current when `update_pointer_state` ran last frame — one Bevy tick
/// behind, since the camera only moves partway through this frame's
/// `Update`). Diffing two such values pulls in that one-frame lag every
/// single frame, which doesn't cancel out — it compounds into a
/// odd/even-frame split that shows up as a visible high-frequency shudder
/// while dragging. Re-projecting the *stored screen* position through the
/// *current* transform each frame (see `handle_camera_pan`) keeps both
/// sides of the subtraction anchored to the same camera state.
#[derive(Resource, Default, Clone, Copy)]
pub enum CameraPanState {
    #[default]
    Idle,
    Pressed {
        start_cursor: Vec2,
    },
    Panning {
        last_screen_pos: Vec2,
        source: PanSource,
    },
}

impl CameraPanState {
    /// Whether the camera is currently being panned — used to suppress the
    /// Edit-mode hover highlight, which has nothing useful to show while
    /// the canvas itself is being dragged around.
    pub fn is_panning(&self) -> bool {
        matches!(self, CameraPanState::Panning { .. })
    }
}

struct PinchAnchor {
    midpoint: Vec2,
    distance: f32,
}

/// The two-finger pinch gesture's own frame-to-frame anchor. Separate from
/// `CameraPanState` because a pinch inherently needs both touches, while
/// `PointerState` (by design, see `pointer.rs`) only ever tracks one.
/// `None` whenever fewer or more than exactly two touches are pressed.
#[derive(Resource, Default)]
pub struct PinchState(Option<PinchAnchor>);

/// True when `world_pos` lands on a placed component or a cable — the test
/// `handle_camera_pan` uses to tell "empty space, free to pan" from
/// "something's there, leave it to selection/placement/drag". Deliberately
/// reimplemented here as a plain read-only lookup rather than reusing
/// `pick_entity_at_cell`: that helper mutates `PickCycleState` to cycle
/// through overlapping entities on repeated clicks, and calling it a second
/// time per click (once here, once in whichever mode-specific handler also
/// runs this frame) would desync that cycle.
fn is_occupied(
    world_pos: Vec2,
    positions: &Query<&GridPosition>,
    cables: &Query<(Entity, &Cable), Without<ChipBridgeCable>>,
) -> bool {
    let cell = world_to_cell(world_pos);
    positions.iter().any(|position| position.0 == cell)
        || find_cable_at(world_pos, cables).is_some()
}

/// Returns the camera translation that keeps `anchor_world` fixed under the
/// same screen position after `OrthographicProjection::scale` changes from
/// `old_scale` to `new_scale` — the standard "zoom toward a point" formula.
/// Deliberately pure algebra rather than mutating `Projection` and calling
/// `viewport_to_world_2d` a second time: neither `GlobalTransform` nor
/// `Camera`'s cached projection matrix refresh until `PostUpdate`, so a
/// same-system read after the mutation would see last frame's stale values.
fn zoom_around(old_translation: Vec2, old_scale: f32, new_scale: f32, anchor_world: Vec2) -> Vec2 {
    anchor_world + (old_translation - anchor_world) * (new_scale / old_scale)
}

/// Drives `CameraPanState` from the middle mouse button and/or the primary
/// pointer, and applies the resulting translation. Suspends the
/// pointer-sourced branch while a two-finger touch gesture is active (see
/// `handle_camera_pinch_zoom`) so the two never fight over the same frame's
/// translation — `PointerState` keeps tracking the first finger throughout,
/// so without this guard a pinch would also register as an ordinary
/// single-pointer pan on top of its own midpoint-based one.
#[allow(clippy::too_many_arguments)]
pub fn handle_camera_pan(
    mouse: Res<ButtonInput<MouseButton>>,
    pointer: Res<PointerState>,
    touches: Res<Touches>,
    armed: Res<ArmedTool>,
    pointer_over_ui: Res<PointerOverUi>,
    positions: Query<&GridPosition>,
    cables: Query<(Entity, &Cable), Without<ChipBridgeCable>>,
    mut pan: ResMut<CameraPanState>,
    camera_query: Single<(&Camera, &GlobalTransform, &mut Transform), With<Camera2d>>,
) {
    let (camera, camera_transform, mut transform) = camera_query.into_inner();

    if touches.iter().count() >= 2
        && !matches!(
            *pan,
            CameraPanState::Panning {
                source: PanSource::Middle,
                ..
            }
        )
    {
        *pan = CameraPanState::Idle;
    }

    if mouse.just_pressed(MouseButton::Middle)
        && let Some(screen_pos) = pointer.screen_pos
    {
        *pan = CameraPanState::Panning {
            last_screen_pos: screen_pos,
            source: PanSource::Middle,
        };
    } else if pointer.just_pressed
        && armed.0.is_none()
        && !pointer_over_ui.0
        && let Some(world_pos) = pointer.world_pos
        && !is_occupied(world_pos, &positions, &cables)
    {
        *pan = CameraPanState::Pressed {
            start_cursor: world_pos,
        };
    }

    match *pan {
        CameraPanState::Idle => {}
        CameraPanState::Pressed { start_cursor } => {
            if !pointer.pressed {
                *pan = CameraPanState::Idle;
            } else if let Some(world_pos) = pointer.world_pos
                && let Some(screen_pos) = pointer.screen_pos
                && start_cursor.distance(world_pos) > EDIT_DRAG_THRESHOLD
            {
                *pan = CameraPanState::Panning {
                    last_screen_pos: screen_pos,
                    source: PanSource::Pointer,
                };
            }
        }
        CameraPanState::Panning {
            last_screen_pos,
            source,
        } => {
            let still_pressed = match source {
                PanSource::Middle => mouse.pressed(MouseButton::Middle),
                PanSource::Pointer => pointer.pressed,
            };
            if !still_pressed {
                *pan = CameraPanState::Idle;
            } else if let Some(current_world) = pointer.world_pos
                && let Some(current_screen) = pointer.screen_pos
                && let Ok(last_world_reprojected) =
                    camera.viewport_to_world_2d(camera_transform, last_screen_pos)
            {
                transform.translation -= (current_world - last_world_reprojected).extend(0.0);
                *pan = CameraPanState::Panning {
                    last_screen_pos: current_screen,
                    source,
                };
            }
        }
    }
}

/// Mouse-wheel zoom, anchored on the cursor so the point under it stays put.
pub fn handle_camera_wheel_zoom(
    mut wheel_events: MessageReader<MouseWheel>,
    pointer: Res<PointerState>,
    mut camera_query: Single<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let scroll: f32 = wheel_events
        .read()
        .map(|event| match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / CAMERA_WHEEL_PIXELS_PER_LINE,
        })
        .sum();
    if scroll == 0.0 {
        return;
    }
    let Some(anchor_world) = pointer.world_pos else {
        return;
    };
    let (transform, projection) = &mut *camera_query;
    let Projection::Orthographic(ortho) = projection.as_mut() else {
        return;
    };

    let old_scale = ortho.scale;
    let new_scale = (old_scale * (1.0 - scroll * CAMERA_WHEEL_ZOOM_SENSITIVITY))
        .clamp(CAMERA_ZOOM_MIN_SCALE, CAMERA_ZOOM_MAX_SCALE);
    let new_translation = zoom_around(
        transform.translation.truncate(),
        old_scale,
        new_scale,
        anchor_world,
    );
    transform.translation = new_translation.extend(transform.translation.z);
    ortho.scale = new_scale;
}

/// Two-finger pinch: zooms around the pinch midpoint (same `zoom_around`
/// primitive as the wheel) and additionally pans by however much that
/// midpoint itself moved on screen, so dragging two fingers together without
/// changing their spread pans instead of doing nothing.
pub fn handle_camera_pinch_zoom(
    touches: Res<Touches>,
    camera: Single<(&Camera, &GlobalTransform, &mut Transform, &mut Projection), With<Camera2d>>,
    mut pinch: ResMut<PinchState>,
) {
    let active: Vec<Vec2> = touches.iter().map(|touch| touch.position()).collect();
    let [a, b] = active.as_slice() else {
        pinch.0 = None;
        return;
    };
    let midpoint = (*a + *b) / 2.0;
    let distance = a.distance(*b);

    let previous = pinch.0.replace(PinchAnchor { midpoint, distance });
    let Some(PinchAnchor {
        midpoint: last_midpoint,
        distance: last_distance,
    }) = previous
    else {
        return;
    };
    if last_distance <= f32::EPSILON {
        return;
    }

    let (camera, camera_transform, mut transform, mut projection) = camera.into_inner();
    let Projection::Orthographic(ortho) = projection.as_mut() else {
        return;
    };
    let (Ok(anchor_world_last), Ok(world_now_at_old_scale)) = (
        camera.viewport_to_world_2d(camera_transform, last_midpoint),
        camera.viewport_to_world_2d(camera_transform, midpoint),
    ) else {
        return;
    };

    let old_scale = ortho.scale;
    let new_scale = (old_scale * (last_distance / distance))
        .clamp(CAMERA_ZOOM_MIN_SCALE, CAMERA_ZOOM_MAX_SCALE);
    let zoomed = zoom_around(
        transform.translation.truncate(),
        old_scale,
        new_scale,
        world_now_at_old_scale,
    );
    let new_translation = zoomed + (anchor_world_last - world_now_at_old_scale);
    transform.translation = new_translation.extend(transform.translation.z);
    ortho.scale = new_scale;
}

#[cfg(test)]
mod zoom_around_tests {
    use super::*;

    #[test]
    fn unchanged_scale_leaves_translation_untouched() {
        let translation = Vec2::new(120.0, -45.0);
        let anchor = Vec2::new(10.0, 10.0);
        assert_eq!(zoom_around(translation, 1.0, 1.0, anchor), translation);
    }

    #[test]
    fn anchor_stays_at_the_same_position_relative_to_the_camera() {
        let old_translation = Vec2::new(-30.0, 5.0);
        let anchor = Vec2::new(50.0, -20.0);
        let (old_scale, new_scale) = (1.0, 0.5);
        let new_translation = zoom_around(old_translation, old_scale, new_scale, anchor);

        // (world - translation) / scale is the anchor's position in the
        // camera's own (unscaled) frame — that's exactly what should stay
        // fixed across a zoom centered on it.
        let relative_before = (anchor - old_translation) / old_scale;
        let relative_after = (anchor - new_translation) / new_scale;
        assert_eq!(relative_before, relative_after);
    }

    #[test]
    fn zooming_in_moves_camera_toward_the_anchor() {
        let translation = Vec2::new(100.0, 0.0);
        let anchor = Vec2::ZERO;
        let new_translation = zoom_around(translation, 1.0, 0.5, anchor);
        assert_eq!(new_translation, Vec2::new(50.0, 0.0));
    }
}
