use bevy::prelude::*;

use crate::constants::FIXED_TICK_SECONDS;

use super::components::SignalWriteBuffer;
use super::net_resolution::stage_net_resolution;
use super::systems::{apply_signal_writes, stage_gate_evaluation};

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Time::<Fixed>::from_seconds(FIXED_TICK_SECONDS))
            .init_resource::<SignalWriteBuffer>()
            .add_systems(
                FixedUpdate,
                (
                    stage_net_resolution,
                    apply_signal_writes,
                    stage_gate_evaluation,
                    apply_signal_writes,
                )
                    .chain(),
            );
    }
}
