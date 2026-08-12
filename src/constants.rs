use bevy::color::Color;

pub const GRID_CELL_SIZE: f32 = 48.0;
pub const FIXED_TICK_SECONDS: f64 = 0.15;
pub const LAMP_MAX: f32 = 1.0;
pub const LABEL_FONT_SIZE: f32 = 14.0;
/// Cursor movement (in pixels) past which a held click in Edit mode counts as
/// a drag (move) rather than a plain click (delete).
pub const EDIT_DRAG_THRESHOLD: f32 = 6.0;
/// How close (in pixels) a click must land to a wire's line to select it in Edit mode.
pub const WIRE_HIT_DISTANCE: f32 = 6.0;

pub const COLOR_HIGH: Color = Color::srgb(1.0, 0.35, 0.15);
pub const COLOR_LOW: Color = Color::srgb(0.2, 0.45, 1.0);
pub const COLOR_NEUTRAL: Color = Color::srgb(0.4, 0.4, 0.45);

pub const COLOR_SWITCH: Color = Color::srgb(0.75, 0.75, 0.2);
pub const COLOR_GATE: Color = Color::srgb(0.3, 0.3, 0.35);
pub const COLOR_LAMP_OFF: Color = Color::srgb(0.25, 0.2, 0.1);

pub const COLOR_BUTTON_NORMAL: Color = Color::srgb(0.15, 0.15, 0.18);
pub const COLOR_BUTTON_ARMED: Color = Color::srgb(0.2, 0.5, 0.25);
pub const COLOR_BUTTON_BORDER: Color = Color::srgb(0.6, 0.6, 0.65);
