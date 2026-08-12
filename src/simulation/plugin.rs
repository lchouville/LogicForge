use bevy::prelude::*;

use crate::constants::FIXED_TICK_SECONDS;

use super::components::SignalWriteBuffer;
use super::systems::{apply_signal_writes, stage_gate_evaluation, stage_wire_propagation};

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_seconds(FIXED_TICK_SECONDS))
            .init_resource::<SignalWriteBuffer>()
            .add_systems(
                FixedUpdate,
                (
                    stage_wire_propagation,
                    apply_signal_writes,
                    stage_gate_evaluation,
                    apply_signal_writes,
                )
                    .chain(),
            );
    }
}
