use bevy::math::{IVec2, Vec2};

use crate::constants::GRID_CELL_SIZE;

pub fn cell_to_world(cell: IVec2) -> Vec2 {
    Vec2::new(
        cell.x as f32 * GRID_CELL_SIZE,
        cell.y as f32 * GRID_CELL_SIZE,
    )
}

pub fn world_to_cell(world: Vec2) -> IVec2 {
    IVec2::new(
        (world.x / GRID_CELL_SIZE).round() as i32,
        (world.y / GRID_CELL_SIZE).round() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_grid_and_world_positions() {
        let cell = IVec2::new(3, -2);
        assert_eq!(world_to_cell(cell_to_world(cell)), cell);
    }

    #[test]
    fn snaps_nearby_world_positions_to_the_same_cell() {
        let base = cell_to_world(IVec2::new(1, 1));
        let nudged = base + Vec2::new(5.0, -5.0);
        assert_eq!(world_to_cell(nudged), IVec2::new(1, 1));
    }
}
