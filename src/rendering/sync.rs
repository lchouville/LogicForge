use bevy::color::Srgba;
use bevy::prelude::*;

use crate::constants::{COLOR_HIGH, COLOR_LAMP_OFF, COLOR_LOW, COLOR_NEUTRAL, LAMP_MAX};
use crate::simulation::components::{Lamp, Pin, SignalValue};
use crate::simulation::logic::{LogicState, read_analog, read_logic};

pub fn sync_pin_colors(mut pins: Query<(&SignalValue, &mut Sprite), With<Pin>>) {
    for (signal, mut sprite) in &mut pins {
        sprite.color = match read_logic(signal.0) {
            LogicState::High => COLOR_HIGH,
            LogicState::Low => COLOR_LOW,
            LogicState::Neutral => COLOR_NEUTRAL,
        };
    }
}

pub fn sync_lamp_brightness(
    mut lamps: Query<(&Children, &mut Sprite), With<Lamp>>,
    pins: Query<&SignalValue, With<Pin>>,
) {
    let off: Srgba = COLOR_LAMP_OFF.to_srgba();
    let lit: Srgba = COLOR_HIGH.to_srgba();

    for (children, mut sprite) in &mut lamps {
        let intensity = children
            .iter()
            .filter_map(|child| pins.get(child).ok())
            .map(|signal| read_analog(signal.0, LAMP_MAX))
            .fold(0.0_f32, f32::max);

        sprite.color = Color::srgba(
            off.red + (lit.red - off.red) * intensity,
            off.green + (lit.green - off.green) * intensity,
            off.blue + (lit.blue - off.blue) * intensity,
            1.0,
        );
    }
}
