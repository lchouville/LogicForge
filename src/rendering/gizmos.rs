use bevy::prelude::*;

use crate::constants::{COLOR_HIGH, COLOR_LOW, COLOR_NEUTRAL};
use crate::grid::cell_to_world;
use crate::simulation::components::{Cable, SignalValue};
use crate::simulation::logic::{LogicState, read_logic};

pub fn draw_cables(cables: Query<(&Cable, &SignalValue)>, mut gizmos: Gizmos) {
    for (cable, signal) in &cables {
        let color = match read_logic(signal.0) {
            LogicState::High => COLOR_HIGH,
            LogicState::Low => COLOR_LOW,
            LogicState::Neutral => COLOR_NEUTRAL,
        };
        gizmos.line_2d(cell_to_world(cable.start), cell_to_world(cable.end), color);
    }
}
