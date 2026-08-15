use bevy::prelude::*;
use bevy::window::Window;

use crate::constants::{BACKGROUND_GRID_ALPHA, BACKGROUND_GRID_Z};
use crate::grid::{cell_to_world, world_to_cell};

use super::appearance::PendingAppearance;

/// One tile of the always-visible workspace grid (the "quadrillage de
/// fond" reference: `node.json` tiled edge-to-edge behind everything).
#[derive(Component)]
pub(crate) struct BackgroundGridTile;

/// Extra cells kept alive past the camera's visible edge so a tile is
/// already in place rather than popping in right as it scrolls into view
/// (relevant once panning exists — today the camera never moves).
const MARGIN_CELLS: i32 = 2;

/// Fills the camera's visible area with `BackgroundGridTile`s reusing
/// `node.json`, growing a pooled set of entities (never despawning, just
/// hiding) as the window/viewport grows. The expensive part — recomputing
/// which cells are visible and repositioning the pool — only reruns when
/// the window actually changes size (or on the very first run); the camera
/// is static today, so there's nothing else that could invalidate it yet —
/// a future pan/zoom system will need to add itself to that condition. The
/// per-tile tint refresh always runs (cheap, same pattern as
/// `sync_pin_colors`) because `apply_loaded_appearances` replaces each
/// tile's `Sprite` outright — with an opaque default color — the moment its
/// `node.json` finishes loading asynchronously, which can land on any
/// frame after the layout was last built.
pub fn sync_background_grid(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    resized_windows: Query<&Window, Changed<Window>>,
    mut ran_once: Local<bool>,
    mut pool: Local<Vec<Entity>>,
    mut tiles: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<BackgroundGridTile>>,
) {
    let (camera, camera_transform) = *camera_query;

    if !*ran_once || !resized_windows.is_empty() {
        let bounds = camera.logical_viewport_size().and_then(|viewport_size| {
            let top_left = camera.viewport_to_world_2d(camera_transform, Vec2::ZERO).ok()?;
            let bottom_right = camera
                .viewport_to_world_2d(camera_transform, viewport_size)
                .ok()?;
            Some((top_left, bottom_right))
        });

        if let Some((top_left, bottom_right)) = bounds {
            let min_world = top_left.min(bottom_right);
            let max_world = top_left.max(bottom_right);
            let min_cell = world_to_cell(min_world) - IVec2::splat(MARGIN_CELLS);
            let max_cell = world_to_cell(max_world) + IVec2::splat(MARGIN_CELLS);
            let cols = (max_cell.x - min_cell.x + 1).max(0);
            let rows = (max_cell.y - min_cell.y + 1).max(0);
            let needed = (cols * rows) as usize;

            while pool.len() < needed {
                let entity = commands
                    .spawn((
                        BackgroundGridTile,
                        Sprite::default(),
                        PendingAppearance(asset_server.load("appearances/node.json")),
                        Transform::from_xyz(0.0, 0.0, BACKGROUND_GRID_Z),
                    ))
                    .id();
                pool.push(entity);
            }

            for (i, &entity) in pool.iter().enumerate() {
                let Ok((_, mut transform, mut visibility)) = tiles.get_mut(entity) else {
                    continue;
                };
                if i < needed {
                    let ix = i as i32 % cols;
                    let iy = i as i32 / cols;
                    let cell = IVec2::new(min_cell.x + ix, min_cell.y + iy);
                    transform.translation = cell_to_world(cell).extend(BACKGROUND_GRID_Z);
                    *visibility = Visibility::Visible;
                } else {
                    *visibility = Visibility::Hidden;
                }
            }

            *ran_once = true;
        }
    }

    for &entity in pool.iter() {
        if let Ok((mut sprite, _, visibility)) = tiles.get_mut(entity)
            && *visibility == Visibility::Visible
        {
            sprite.color = Color::srgba(1.0, 1.0, 1.0, BACKGROUND_GRID_ALPHA);
        }
    }
}
