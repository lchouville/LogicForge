use bevy::prelude::*;

use super::cursor::cursor_world_position;

/// Unifies mouse and (single-finger) touch input behind one press/position
/// stream, so every click/drag system needs to check only one source instead
/// of duplicating itself per input device. A tracked touch always wins over
/// the mouse for the frames it's active — a hybrid mouse+touch device never
/// mixes the two mid-gesture.
#[derive(Resource, Default, Clone, Copy)]
pub struct PointerState {
    pub just_pressed: bool,
    pub pressed: bool,
    pub just_released: bool,
    pub world_pos: Option<Vec2>,
    /// Raw pre-projection screen position (window cursor / touch position).
    /// World positions computed on different frames aren't directly
    /// comparable — the camera may have moved between them — so anything
    /// that accumulates a delta across frames (camera panning) needs to
    /// re-project a *stored* screen position through the *current* camera
    /// transform each frame rather than diffing two already-projected
    /// `world_pos` values from different frames. See `camera_control.rs`.
    pub screen_pos: Option<Vec2>,
}

/// The touch id currently driving `PointerState`, remembered across frames so
/// a multi-frame drag keeps following the same finger. Multi-touch gestures
/// (pinch/pan) are out of scope here — see the mobile pan/zoom roadmap item —
/// so any touch beyond the first one adopted is ignored until it becomes the
/// tracked one itself.
#[derive(Resource, Default)]
pub struct ActiveTouch(Option<u64>);

pub fn update_pointer_state(
    mouse: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    mut active_touch: ResMut<ActiveTouch>,
    window: Single<&Window>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    mut pointer: ResMut<PointerState>,
) {
    let (camera, camera_transform) = *camera_query;

    if active_touch.0.is_none()
        && let Some(touch) = touches.iter_just_pressed().next()
    {
        active_touch.0 = Some(touch.id());
    }

    if let Some(id) = active_touch.0 {
        if let Some(touch) = touches.get_pressed(id) {
            let screen_pos = touch.position();
            *pointer = PointerState {
                just_pressed: touches.just_pressed(id),
                pressed: true,
                just_released: false,
                world_pos: camera
                    .viewport_to_world_2d(camera_transform, screen_pos)
                    .ok(),
                screen_pos: Some(screen_pos),
            };
        } else {
            // Ended or canceled this frame: stop tracking it and report the
            // release once, same as a mouse button-up.
            active_touch.0 = None;
            let screen_pos = touches.get_released(id).map(|touch| touch.position());
            *pointer = PointerState {
                just_pressed: false,
                pressed: false,
                just_released: true,
                world_pos: screen_pos.and_then(|screen_pos| {
                    camera
                        .viewport_to_world_2d(camera_transform, screen_pos)
                        .ok()
                }),
                screen_pos,
            };
        }
        return;
    }

    let screen_pos = window.cursor_position();
    *pointer = PointerState {
        just_pressed: mouse.just_pressed(MouseButton::Left),
        pressed: mouse.pressed(MouseButton::Left),
        just_released: mouse.just_released(MouseButton::Left),
        world_pos: cursor_world_position(&window, camera, camera_transform),
        screen_pos,
    };
}
