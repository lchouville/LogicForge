use bevy::prelude::*;

use crate::constants::{CABLE_Z, PIXEL_GRID_DIM, PIXEL_UNIT};
use crate::grid::cell_to_world;
use crate::simulation::components::{Cable, SignalValue};
use crate::simulation::logic::read_logic;

use super::appearance::PendingAppearance;
use super::sync::signal_color;

/// Which end of the cable a `CableEndpoint` child sits at, so
/// `sync_cable_sprite` knows whether to place it at `Cable::start` or
/// `Cable::end`.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CableEndSlot {
    Start,
    End,
}

/// Marks the child that renders the stretched wire strand between the two
/// endpoints.
#[derive(Component)]
pub(crate) struct CableCenter;

/// A cable's endpoint cap: reuses the same `pin.json`/`node.json` art as a
/// component's `Pin`, so a cable reads as "plugged into a grid node" at both
/// ends exactly like a gate/switch/lamp pin does, just without the logical
/// `Pin` component (a cable already carries its own `SignalValue`).
#[derive(Component)]
pub(crate) struct CableEndpoint(CableEndSlot);

fn endpoint(asset_server: &AssetServer, slot: CableEndSlot) -> impl Bundle {
    (
        CableEndpoint(slot),
        Sprite::default(),
        PendingAppearance(asset_server.load("appearances/pin.json")),
        children![(
            Sprite::default(),
            PendingAppearance(asset_server.load("appearances/node.json")),
            Transform::from_xyz(0.0, 0.0, -0.5),
        )],
    )
}

/// Spawns a cable as a fixed parent (never moved; `sync_cable_sprite` drives
/// its children's `Sprite`/`Transform` every frame from `Cable`+
/// `SignalValue`) with three visual children: a stretched/rotated center
/// strand and two endpoint caps, composed from the same reusable pieces as
/// every other pin in the game rather than baked per-signal-state textures.
pub fn spawn_cable(
    commands: &mut Commands,
    asset_server: &AssetServer,
    start: IVec2,
    end: IVec2,
) -> Entity {
    commands
        .spawn((
            Cable { start, end },
            SignalValue::default(),
            Transform::from_xyz(0.0, 0.0, CABLE_Z),
            Visibility::default(),
            children![
                (
                    CableCenter,
                    Sprite::default(),
                    PendingAppearance(asset_server.load("appearances/cable_center.json")),
                ),
                endpoint(asset_server, CableEndSlot::Start),
                endpoint(asset_server, CableEndSlot::End),
            ],
        ))
        .id()
}

/// Drives every cable's center strand and endpoint caps from its `Cable`
/// endpoints and `SignalValue`: tints all three pieces to the current signal
/// color, stretches+rotates the center to span endpoint-to-endpoint (cables
/// aren't constrained to axis-aligned runs, so a single stretched+rotated
/// sprite handles any angle without a multi-segment tiling system), and
/// pins the two endpoint caps exactly on `start`/`end`.
pub fn sync_cable_sprite(
    cables: Query<(&Cable, &SignalValue, &Children)>,
    mut centers: Query<(&mut Sprite, &mut Transform), (With<CableCenter>, Without<CableEndpoint>)>,
    mut endpoints: Query<
        (&CableEndpoint, &mut Sprite, &mut Transform),
        Without<CableCenter>,
    >,
) {
    for (cable, signal, children) in &cables {
        let color = signal_color(read_logic(signal.0));
        let start = cell_to_world(cable.start);
        let end = cell_to_world(cable.end);
        let delta = end - start;
        let length = delta.length().max(1.0);
        let rotation = Quat::from_rotation_z(delta.y.atan2(delta.x));
        let midpoint = start.midpoint(end);

        for &child in children {
            if let Ok((mut sprite, mut transform)) = centers.get_mut(child) {
                sprite.color = color;
                sprite.custom_size = Some(Vec2::new(length, PIXEL_GRID_DIM as f32 * PIXEL_UNIT));
                transform.translation = midpoint.extend(0.0);
                transform.rotation = rotation;
            } else if let Ok((endpoint, mut sprite, mut transform)) = endpoints.get_mut(child) {
                sprite.color = color;
                let position = match endpoint.0 {
                    CableEndSlot::Start => start,
                    CableEndSlot::End => end,
                };
                transform.translation = position.extend(0.1);
            }
        }
    }
}
