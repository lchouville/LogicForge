use bevy::prelude::*;

use crate::constants::{CABLE_BODY_HIT_DISTANCE, CABLE_ENDPOINT_HIT_RADIUS};
use crate::grid::cell_to_world;
use crate::simulation::components::Cable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CableEnd {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CableHit {
    Endpoint(CableEnd),
    Body,
}

/// Shortest distance from `point` to the segment `a`-`b`.
pub fn distance_to_segment(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq <= f32::EPSILON {
        return point.distance(a);
    }
    let t = ((point - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    point.distance(a + ab * t)
}

/// Finds the cable (if any) closest to `world_pos`, classifying the hit as
/// one of its two endpoints (within `CABLE_ENDPOINT_HIT_RADIUS`, checked
/// first so an endpoint near the body still grabs the endpoint) or its body
/// (within `CABLE_BODY_HIT_DISTANCE`), so Edit mode can tell a whole-cable
/// move from an endpoint reshape.
pub fn find_cable_at(
    world_pos: Vec2,
    cables: &Query<(Entity, &Cable)>,
) -> Option<(Entity, CableHit)> {
    cables
        .iter()
        .filter_map(|(entity, cable)| {
            let start = cell_to_world(cable.start);
            let end = cell_to_world(cable.end);

            let start_distance = world_pos.distance(start);
            let end_distance = world_pos.distance(end);
            if start_distance <= CABLE_ENDPOINT_HIT_RADIUS
                && start_distance <= end_distance
            {
                return Some((entity, CableHit::Endpoint(CableEnd::Start), start_distance));
            }
            if end_distance <= CABLE_ENDPOINT_HIT_RADIUS {
                return Some((entity, CableHit::Endpoint(CableEnd::End), end_distance));
            }

            let body_distance = distance_to_segment(world_pos, start, end);
            (body_distance <= CABLE_BODY_HIT_DISTANCE)
                .then_some((entity, CableHit::Body, body_distance))
        })
        .min_by(|(_, _, a), (_, _, b)| a.total_cmp(b))
        .map(|(entity, hit, _)| (entity, hit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_to_segment_is_zero_on_the_line() {
        let midpoint = Vec2::new(5.0, 0.0);
        assert_eq!(
            distance_to_segment(midpoint, Vec2::ZERO, Vec2::new(10.0, 0.0)),
            0.0
        );
    }

    #[test]
    fn distance_to_segment_clamps_to_the_nearest_endpoint() {
        let beyond_b = Vec2::new(15.0, 0.0);
        assert_eq!(
            distance_to_segment(beyond_b, Vec2::ZERO, Vec2::new(10.0, 0.0)),
            5.0
        );
    }

    #[test]
    fn distance_to_segment_measures_perpendicular_offset() {
        let above_midpoint = Vec2::new(5.0, 3.0);
        assert_eq!(
            distance_to_segment(above_midpoint, Vec2::ZERO, Vec2::new(10.0, 0.0)),
            3.0
        );
    }

    #[test]
    fn distance_to_degenerate_segment_is_distance_to_point() {
        let point = Vec2::new(3.0, 4.0);
        assert_eq!(distance_to_segment(point, Vec2::ZERO, Vec2::ZERO), 5.0);
    }
}
