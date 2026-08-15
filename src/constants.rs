use bevy::color::Color;

pub const GRID_CELL_SIZE: f32 = 48.0;
/// Side length, in texels, of one pixel-art "block" — the unit every
/// appearance JSON's width/height must be a multiple of (see
/// `src/rendering/appearance.rs`). A component's canvas can span several
/// blocks (e.g. a 2-cell-tall gate body), but each block always maps onto
/// exactly one grid cell.
pub const PIXEL_GRID_DIM: usize = 16;
/// World units per texel: a `PIXEL_GRID_DIM`-wide block renders at exactly
/// `GRID_CELL_SIZE`, keeping every pixel-art asset grid-aligned regardless
/// of its native resolution.
pub const PIXEL_UNIT: f32 = GRID_CELL_SIZE / PIXEL_GRID_DIM as f32;
/// The simulation's time model: 1 tick = 1ms. Net/wire connectivity has no
/// propagation delay of its own — a net (see `stage_net_resolution`) is
/// fully resolved within the tick it's computed in, so a wire "lights up"
/// instantly. Each *gate* a signal passes through does cost one tick of
/// latency, though: `stage_net_resolution` (which feeds every gate's
/// inputs) runs before `stage_gate_evaluation` in the same tick, so a
/// gate's newly-computed output isn't visible to net resolution until the
/// *next* tick. At 1ms that's imperceptible to a player even through a long
/// chain of gates, while still giving the simulation a well-defined,
/// per-gate notion of propagation delay — relevant later for anything that
/// cares about ordering or timing (the Horloge/Clock, sequential logic).
pub const FIXED_TICK_SECONDS: f64 = 0.001;
pub const LAMP_MAX: f32 = 1.0;
pub const LABEL_FONT_SIZE: f32 = 14.0;
/// Cursor movement (in pixels) past which a held click in Edit mode counts as
/// a drag (move) rather than a plain click (delete).
pub const EDIT_DRAG_THRESHOLD: f32 = 6.0;
/// How close (in pixels) a click must land to a cable's line to select its
/// body (as opposed to one of its endpoints) in Edit mode.
pub const CABLE_BODY_HIT_DISTANCE: f32 = 6.0;
/// How close (in pixels) a click must land to a cable's start/end point to
/// grab that endpoint specifically, instead of the cable's body.
pub const CABLE_ENDPOINT_HIT_RADIUS: f32 = 10.0;
/// Z-offset handed out per placed component so overlapping bodies (overlap
/// is allowed, needed for the connect-by-contact mechanic) still draw in a
/// deterministic, later-placed-on-top order.
pub const SPAWN_Z_STEP: f32 = 0.01;
/// Fixed z for every cable sprite, behind all placed components regardless
/// of `SPAWN_Z_STEP` order.
pub const CABLE_Z: f32 = -100.0;
/// Fixed z for the tiled background grid — behind cables too, so it never
/// competes with anything the player actually places.
pub const BACKGROUND_GRID_Z: f32 = -200.0;
/// Opacity of each background grid tile: low enough to read as a subtle
/// backdrop (graph-paper/PCB reference grid) rather than compete visually
/// with placed components.
pub const BACKGROUND_GRID_ALPHA: f32 = 0.35;
/// Fixed z for the armed-tool placement-preview ghost — above everything
/// else (placed components, cables, the background grid) so it's always
/// clearly visible regardless of what's already on the cell underneath.
/// `SPAWN_Z_STEP`-incrementing real components stay well under 100 even for
/// a very large circuit, so this has a huge margin below it; deliberately
/// NOT pushed any higher than that, because Bevy's default 2D camera clips
/// at z=1000 — a previous value of 1000.0 here put pin/label children (at a
/// small *positive* local z on top of the ghost's own root z) just past
/// that far plane, silently culling them despite spawning correctly.
pub const PREVIEW_Z: f32 = 500.0;
/// Opacity of the placement-preview ghost: dim enough to read unmistakably
/// as "not placed yet" next to a real, fully-opaque component.
pub const PREVIEW_ALPHA: f32 = 0.45;

pub const COLOR_HIGH: Color = Color::srgb(1.0, 0.35, 0.15);
pub const COLOR_LOW: Color = Color::srgb(0.2, 0.45, 1.0);
pub const COLOR_NEUTRAL: Color = Color::srgb(0.4, 0.4, 0.45);

pub const COLOR_SWITCH: Color = Color::srgb(0.75, 0.75, 0.2);
pub const COLOR_GATE: Color = Color::srgb(0.3, 0.3, 0.35);
pub const COLOR_LAMP_OFF: Color = Color::srgb(0.25, 0.2, 0.1);

pub const COLOR_BUTTON_NORMAL: Color = Color::srgb(0.15, 0.15, 0.18);
pub const COLOR_BUTTON_ARMED: Color = Color::srgb(0.2, 0.5, 0.25);
pub const COLOR_BUTTON_BORDER: Color = Color::srgb(0.6, 0.6, 0.65);
