use bevy::prelude::*;

use crate::constants::{COLOR_HIGH, COLOR_LOW, COLOR_NEUTRAL};
use crate::simulation::components::{SignalValue, Wire};
use crate::simulation::logic::{LogicState, read_logic};

pub fn draw_wires(
    wires: Query<&Wire>,
    pins: Query<(&GlobalTransform, &SignalValue)>,
    mut gizmos: Gizmos,
) {
    for wire in &wires {
        let Ok((from_transform, from_signal)) = pins.get(wire.from) else {
            continue;
        };
        let Ok((to_transform, _)) = pins.get(wire.to) else {
            continue;
        };

        let color = match read_logic(from_signal.0) {
            LogicState::High => COLOR_HIGH,
            LogicState::Low => COLOR_LOW,
            LogicState::Neutral => COLOR_NEUTRAL,
        };

        gizmos.line_2d(
            from_transform.translation().truncate(),
            to_transform.translation().truncate(),
            color,
        );
    }
}
