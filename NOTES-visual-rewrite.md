# WIP planning notes: pixel-art visuals + cable connectivity rewrite

Not code, not merged into `develop` — a checkpoint of an in-progress `/plan`
discussion so it can be resumed on another machine. Safe to delete this file
once the corresponding feature branch(es) land, or once the plan is finalized
into a proper `.claude/plans/*.md` on the resuming machine.

## Where this came from

After shipping MVP v0.1 (core simulation kernel) and the wasm/GitHub Pages
deploy, the next ask was: "on s'attaque au visuel maintenant" — tackle the
visuals. Turned into two intertwined feature requests, clarified over several
rounds of questions. **No code has been written for any of this yet** — we
were mid-way through validating the architecture (a Plan sub-agent call was
in flight, interrupted to save state) before writing a final plan file and
getting `ExitPlanMode` sign-off.

## Confirmed decisions

### 1. Pixel-art appearance system (JSON-driven)
- Component **bodies** switch from hardcoded flat-color `Sprite`s to a blocky
  pixel-art look.
- Appearance is authored by **hand-editing a JSON file per component** — a
  small grid of pixels, each an index into a color palette. This lets the
  project owner (no art-tool workflow) "draw" an icon by editing indices in a
  JSON array, and is explicitly data-driven so a future theme/mode system can
  swap the whole visual style by pointing at a different file set, without
  touching game logic.
- Rejected alternative: real PNG sprite files (would require actually
  drawing/finding art, not just editing JSON).
- Pins and signal-reactive coloring (lamp brightness, wire/net color) stay
  procedural flat-color squares — those change every frame from live
  simulation state, not static "art", so they don't need to go through the
  JSON pipeline. Their size should just visually harmonize with the new pixel
  grid unit.
- Hard requirement: **no floating/misaligned elements** — everything grid-
  aligned, crisp (no blur from non-integer positions or texture scaling).
  Bevy-side, this likely means `ImagePlugin::default_nearest()` (point/nearest
  sampling) plus choosing pixel-grid dimensions so the rendered body size is
  an exact multiple of `GRID_CELL_SIZE` (currently 48.0, see
  `src/constants.rs`).

### 2. Cables become first-class, path-based placeable elements
Superseding the current `Wire{from:Entity,to:Entity}` drag-between-two-pins
model entirely:
- A cable is placed by **arming a "cable" tool then click-dragging (or
  clicking a sequence of nodes) from a start node across the grid to an end
  node** — the whole traced path becomes **one cable entity** holding an
  **ordered list of grid-node points**.
- Movable as a whole (drag anywhere along it, not near an endpoint, to
  translate every point together) **and its endpoints individually** (drag
  near one end to reshape/re-route just that end, other end stays put). The
  user asked for both; endpoint-specific reshaping was noted as something to
  flag if it meaningfully raises implementation risk enough to warrant
  splitting into a fast-follow rather than bundling with the rest — not yet
  resolved which way that goes.
- **Connectivity is spatial, not an explicit graph edge**: any two
  "connection-bearing" things (a component's `Pin`, or a point along a
  `Cable`'s path) that occupy **the same grid node** are the same electrical
  net — "the pin connects by being superposed (on a component's pin, or on a
  cable)". Explicitly inspired by *Geareo* (gears mesh by touching, not via
  an explicit "connect A to B" action) — **only the connect-by-contact idea**
  was taken from Geareo, not its visual style or gridless placement (Geareo
  itself is a gridless 3D physics sandbox, quite different from LogicForge's
  strict 2D grid — confirmed explicitly with the user this is the one thing
  being borrowed).
- **Components may now overlap** / be placed on an already-occupied cell —
  intentional, needed for the connect-by-overlap mechanic (e.g. placing a new
  component so its pin lands exactly on an existing cable's point). The
  current `is_cell_occupied` placement-blocking check (`src/editor/
  placement.rs`) and the analogous move-blocking check in Edit mode
  (`src/editor/edit_mode.rs`) need to go away.

## Architecture sketch (not yet validated/finalized)

- **Simulation**: replace `Wire` + `stage_wire_propagation` (`src/simulation/
  systems.rs`) with a `Cable{points: Vec<IVec2>}` component (+ its own
  `SignalValue` so its render color can reflect current signal) and a
  net-resolution pass: collect every `Pin`'s grid node + every `Cable`'s node
  list, group into connected "nets" via shared-node adjacency (a simple
  flood-fill is plenty at this entity scale — no need for real union-find),
  propagate any driving `Output` pin's value to the rest of its net. Keep
  this *separate from and before* gate evaluation in tick order, so gates
  still only advance once per `FixedUpdate` tick (preserves the existing
  "visible ripple across ticks, feedback loops just blink instead of
  hanging" property) while wire/net connectivity itself resolves instantly
  within a tick (a real wire has no propagation delay — only gates should).
- **Pixel art**: a small `serde`-deserializable asset struct (palette + pixel
  index grid + pixel size in world units), a hand-written Bevy `AssetLoader`
  for it (avoid pulling in `bevy_common_assets` given wasm-bundle-size
  awareness — just `serde`+`serde_json`), assets under `assets/appearances/`.
  Since Bevy asset loading is async, need a system that detects "this
  component's appearance JSON just finished loading" and generates a
  procedural `Image` texture from it (RGBA8 from palette+pixels) to assign as
  the `Sprite`'s image handle — not yet designed in detail how a
  freshly-spawned component tracks "waiting on my appearance to load".
- Files expected to be touched: `src/simulation/components.rs`, `src/
  simulation/systems.rs` (or a new `net_resolution.rs`), `src/editor/
  spawn.rs`, `src/editor/interaction.rs` (remove old wire-drag gesture),
  `src/editor/placement.rs` (cable tool, remove occupancy blocking), `src/
  editor/edit_mode.rs` (cable move/reshape/delete), `src/rendering/*`,
  `src/constants.rs`, new `assets/appearances/*.json`, likely a new asset-
  loader module.
- Open question flagged but not resolved: whether to ship this as one PR or
  split into two (pixel-art appearances vs. cable/connectivity overhaul) per
  the project's "one branch = one subject" gitflow convention — leaning
  toward splitting, not decided.

## Repo state at the time of this note

`develop`, `main`, and `pages` are all fully pushed to `origin`, nothing
stranded locally. `main` is tagged `v0.1.0`. This branch
(`notes/visual-rewrite-planning`) exists only to carry this file — not
intended to be merged, delete it once superseded by real feature branches.

## Next step on resume

Either re-run the architecture-validation pass (a Plan-agent-style deep
review of the JSON asset-loading pipeline and the node/net connectivity
model above — the previous attempt at this was interrupted before
completing) or go straight to drafting the final plan file, then
`ExitPlanMode` for the project owner's sign-off before writing any code.
