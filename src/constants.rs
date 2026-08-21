use bevy::color::Color;
use bevy::math::Vec2;

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
/// UI text font, vendored under `assets/fonts/` (see `FiraMono-LICENSE`,
/// SIL OFL) — Bevy's built-in default font is an ASCII-only subset (~19KB vs
/// this file's ~170KB) that renders accented characters as missing-glyph
/// boxes, which broke the French inspector-panel text (see item 4 of the
/// roadmap).
pub const UI_FONT_PATH: &str = "fonts/FiraMono-Medium.ttf";
/// Cursor movement (in pixels) past which a held click in Edit mode counts as
/// a drag (move) rather than a plain click (select).
pub const EDIT_DRAG_THRESHOLD: f32 = 6.0;
/// Width of the project sidebar's list body when expanded (see
/// `editor::sidebar`) — the always-visible header/toggle row sizes itself to
/// its own content instead, so it stays reachable while the body is
/// collapsed.
pub const SIDEBAR_WIDTH: f32 = 200.0;
/// World-space translation applied to every chip structure block (see
/// `editor::chip_structure`) so a project's exterior structure lives far
/// away from its interior circuit in the same ECS world, rather than needing
/// to despawn/respawn either one when toggling between the two views — large
/// enough that it's never visible at once with the interior circuit even
/// fully zoomed out (`CAMERA_ZOOM_MAX_SCALE`). Deliberately expressed as a
/// multiple of `GRID_CELL_SIZE` rather than a bare literal: the tiled
/// background grid (`sync_background_grid`) always draws at plain
/// `cell_to_world(cell)`, with no offset of its own, so any offset here that
/// isn't itself a whole number of cells desyncs the two grids — every
/// structure block used to render visibly off the background grid's node
/// icons until this was caught.
pub const STRUCTURE_SPACE_OFFSET: Vec2 = Vec2::new(GRID_CELL_SIZE * 2_084.0, 0.0);
/// World-space region where every placed `ChipInstance`'s private, actually-
/// simulated copy of its source project's interior circuit lives (see
/// `editor::chip_instance::ChipInstanceSlotAllocator`) — on the **Y** axis,
/// deliberately distinct from `STRUCTURE_SPACE_OFFSET`'s X axis, so the two
/// "parked far away" regions can never alias each other even if one offset
/// is later tuned. Same reasoning as `STRUCTURE_SPACE_OFFSET` for staying a
/// whole multiple of `GRID_CELL_SIZE`: keeps every nested circuit grid-
/// aligned with the background grid.
pub const CHIP_INTERIOR_SPACE_OFFSET: Vec2 = Vec2::new(0.0, GRID_CELL_SIZE * 4_096.0);
/// Cell-space stride between two consecutive placed instances' private
/// interior-circuit slots (`ChipInstanceSlotAllocator`) — generous enough
/// that a reasonably-sized source circuit never spills into its neighbor's
/// slot. Allocation is a flat, ever-increasing counter (never reused, never
/// reset, not partitioned by nesting depth), so this same stride also
/// separates a recursively-nested chip's own slot from every other slot,
/// regardless of how deep it's nested.
pub const CHIP_INTERIOR_INSTANCE_STRIDE_CELLS: i32 = 512;
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
/// Placeholder color for the interior `PinHeader` component while its real
/// "PCB pin block" appearance streams in — dark plastic-connector black.
pub const COLOR_PIN_HEADER: Color = Color::srgb(0.12, 0.12, 0.13);

/// Fixed color for a Lampe block in the chip structure editor (see
/// `editor::chip_structure`) — not tinted by `ActiveStructureColor`, since
/// that palette only customizes the Corps (body) blocks. A Pin block has no
/// color of its own to fix here: it reuses the interior circuit's own
/// `pin.json` appearance instead (see `chip_structure::spawn_structure_block`).
pub const COLOR_STRUCTURE_LAMP: Color = Color::srgb(0.95, 0.85, 0.3);
/// Lit tint for a placed `ChipInstance`'s Pin/Lamp socket when it carries a
/// HIGH signal (see `chip_instance::sync_chip_instance_socket_color`) — a
/// deliberately simple placeholder (plain white) rather than a proper glow,
/// to refine later.
pub const COLOR_CHIP_SOCKET_LIT: Color = Color::WHITE;
/// Fixed choices offered for the chip structure's Corps (body) color — see
/// `editor::chip_structure::ActiveStructureColor`.
pub const STRUCTURE_COLOR_PALETTE: [Color; 6] = [
    Color::srgb(0.6, 0.6, 0.65),
    Color::srgb(0.8, 0.25, 0.25),
    Color::srgb(0.25, 0.55, 0.8),
    Color::srgb(0.3, 0.7, 0.35),
    Color::srgb(0.85, 0.6, 0.2),
    Color::srgb(0.55, 0.35, 0.75),
];

pub const COLOR_BUTTON_NORMAL: Color = Color::srgb(0.15, 0.15, 0.18);
pub const COLOR_BUTTON_ARMED: Color = Color::srgb(0.2, 0.5, 0.25);
pub const COLOR_BUTTON_BORDER: Color = Color::srgb(0.6, 0.6, 0.65);

/// Outline color for the currently-selected entity in Edit mode (see
/// `Selected`) — bright magenta, chosen to stand out against every other
/// color in the palette above (grays, yellow switch, red/blue signal tints).
pub const COLOR_SELECTION: Color = Color::srgb(1.0, 0.25, 0.85);
/// Outline color for whatever's under the cursor in Edit mode, before it's
/// clicked (see `render_hover_highlight`) — a dimmer cyan, distinct from
/// `COLOR_SELECTION` so a hovered-but-not-selected element never reads as
/// already selected.
pub const COLOR_HOVER: Color = Color::srgb(0.35, 0.85, 0.95);
/// Extra padding (world units) added around a selected component's footprint
/// so the outline reads as "around" the sprite rather than clipping it.
pub const SELECTION_OUTLINE_MARGIN: f32 = 6.0;

/// Smallest allowed `OrthographicProjection::scale` (most zoomed in) — keeps
/// the wheel/pinch from zooming in until nothing but a giant single sprite is
/// visible.
pub const CAMERA_ZOOM_MIN_SCALE: f32 = 0.25;
/// Largest allowed `OrthographicProjection::scale` (most zoomed out) — keeps
/// the background grid tile pool (see `sync_background_grid`) from having to
/// cover an unbounded viewport.
pub const CAMERA_ZOOM_MAX_SCALE: f32 = 4.0;
/// Multiplicative zoom step applied per notch of `MouseWheel` scroll (e.g.
/// `1.0 - CAMERA_WHEEL_ZOOM_SENSITIVITY` per notch scrolled toward the
/// screen). Exponential rather than additive so the zoom feels consistent at
/// any current scale.
pub const CAMERA_WHEEL_ZOOM_SENSITIVITY: f32 = 0.1;
/// Approximate pixels per scroll "notch", used to normalize
/// `MouseWheel`/`MouseScrollUnit::Pixel` deltas (trackpads, and the wheel
/// events this project's own browser test tooling dispatches) down to the
/// same effective step size as `MouseScrollUnit::Line` deltas (physical
/// mouse wheels, ~1 unit per notch) before applying
/// `CAMERA_WHEEL_ZOOM_SENSITIVITY` — without this, a single Pixel-unit
/// scroll event is ~100x a Line-unit one and instantly slams the zoom into
/// its clamped bounds.
pub const CAMERA_WHEEL_PIXELS_PER_LINE: f32 = 100.0;
