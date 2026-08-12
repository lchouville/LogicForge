use bevy::prelude::*;

use crate::constants::WIRE_HIT_DISTANCE;
use crate::simulation::components::{Pin, PinRole, Wire};

const PIN_HIT_RADIUS: f32 = 10.0;

pub fn is_valid_wire_target(
    source_role: PinRole,
    target_role: PinRole,
    target_already_wired: bool,
) -> bool {
    source_role == PinRole::Output && target_role == PinRole::Input && !target_already_wired
}

pub fn is_pin_wired(target_pin: Entity, wires: &Query<&Wire>) -> bool {
    wires.iter().any(|wire| wire.to == target_pin)
}

pub fn find_pin_at(
    world_pos: Vec2,
    pins: &Query<(Entity, &Pin, &GlobalTransform)>,
) -> Option<(Entity, PinRole)> {
    pins.iter()
        .map(|(entity, pin, transform)| {
            let distance = transform.translation().truncate().distance(world_pos);
            (entity, pin.role, distance)
        })
        .filter(|(_, _, distance)| *distance <= PIN_HIT_RADIUS)
        .min_by(|(_, _, a), (_, _, b)| a.total_cmp(b))
        .map(|(entity, role, _)| (entity, role))
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

/// Finds the wire (if any) whose line passes closest to `world_pos`, within
/// `WIRE_HIT_DISTANCE`, so it can be selected/deleted in Edit mode.
pub fn find_wire_at(
    world_pos: Vec2,
    wires: &Query<(Entity, &Wire)>,
    pins: &Query<&GlobalTransform, With<Pin>>,
) -> Option<Entity> {
    wires
        .iter()
        .filter_map(|(entity, wire)| {
            let from = pins.get(wire.from).ok()?.translation().truncate();
            let to = pins.get(wire.to).ok()?.translation().truncate();
            let distance = distance_to_segment(world_pos, from, to);
            (distance <= WIRE_HIT_DISTANCE).then_some((entity, distance))
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(entity, _)| entity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_must_start_at_an_output_pin() {
        assert!(!is_valid_wire_target(PinRole::Input, PinRole::Input, false));
    }

    #[test]
    fn wire_must_end_at_an_input_pin() {
        assert!(!is_valid_wire_target(
            PinRole::Output,
            PinRole::Output,
            false
        ));
    }

    #[test]
    fn wire_cannot_target_an_already_wired_input() {
        assert!(!is_valid_wire_target(PinRole::Output, PinRole::Input, true));
    }

    #[test]
    fn output_to_free_input_is_valid() {
        assert!(is_valid_wire_target(PinRole::Output, PinRole::Input, false));
    }

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
