use bevy::prelude::*;

use super::components::{GateKind, Pin, PinRole, SignalValue, SignalWriteBuffer, Wire};
use super::logic::{eval_and, eval_not, eval_or};

pub fn stage_wire_propagation(
    wires: Query<&Wire>,
    signals: Query<&SignalValue>,
    mut buffer: ResMut<SignalWriteBuffer>,
) {
    for wire in &wires {
        if let Ok(value) = signals.get(wire.from) {
            buffer.0.push((wire.to, value.0));
        }
    }
}

pub fn apply_signal_writes(
    mut buffer: ResMut<SignalWriteBuffer>,
    mut signals: Query<&mut SignalValue>,
) {
    for (entity, value) in buffer.0.drain(..) {
        if let Ok(mut signal) = signals.get_mut(entity) {
            signal.0 = value;
        }
    }
}

pub fn stage_gate_evaluation(
    gates: Query<(&GateKind, &Children)>,
    pins: Query<(&Pin, &SignalValue)>,
    mut buffer: ResMut<SignalWriteBuffer>,
) {
    for (kind, children) in &gates {
        let mut inputs = [0.0_f32; 2];
        let mut output_pin = None;
        for child in children.iter() {
            let Ok((pin, signal)) = pins.get(child) else {
                continue;
            };
            match pin.role {
                PinRole::Input => {
                    if let Some(slot) = inputs.get_mut(pin.index as usize) {
                        *slot = signal.0;
                    }
                }
                PinRole::Output => output_pin = Some(child),
            }
        }

        let Some(output_pin) = output_pin else {
            continue;
        };

        let result = match kind {
            GateKind::And => eval_and(inputs[0], inputs[1]),
            GateKind::Or => eval_or(inputs[0], inputs[1]),
            GateKind::Not => eval_not(inputs[0]),
        };

        buffer.0.push((output_pin, result));
    }
}
