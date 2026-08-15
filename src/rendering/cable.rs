use bevy::prelude::*;

use crate::constants::{CABLE_Z, GRID_CELL_SIZE};
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

/// Marks one fixed-size, never-stretched wire "bead" (`cable_center.json`).
/// A cable is tiled end-to-end with as many of these as fit its current
/// start-end span (see `rebuild_cable_segments`) instead of non-uniformly
/// stretching a single sprite, so the art never gets squashed the way it
/// would for arbitrary distances — it matches the reference's fixed-size
/// capsule icon regardless of how far apart the two ends are.
#[derive(Component)]
pub(crate) struct CableSegment;

/// A cable's endpoint cap: reuses the same `pin.json` art as a component's
/// `Pin`, so a cable reads as "plugged into a grid node" at both ends
/// exactly like a gate/switch/lamp pin does, just without the logical `Pin`
/// component (a cable already carries its own `SignalValue`). No
/// `node.json` bracket child — the background grid already tiles that
/// under every cell, so a per-endpoint copy was a redundant extra frame.
#[derive(Component)]
pub(crate) struct CableEndpoint(CableEndSlot);

fn endpoint(asset_server: &AssetServer, slot: CableEndSlot) -> impl Bundle {
    (
        CableEndpoint(slot),
        Sprite::default(),
        PendingAppearance(asset_server.load("appearances/pin.json")),
    )
}

/// Spawns a cable as a fixed parent (never moved directly; `Cable::start`/
/// `end` drive everything) with two endpoint caps as children. Its wire-body
/// `CableSegment` tiles are populated separately by `rebuild_cable_segments`
/// once `Cable` is readable (needs `Changed<Cable>`, which only fires once
/// the component actually exists).
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
                endpoint(asset_server, CableEndSlot::Start),
                endpoint(asset_server, CableEndSlot::End),
            ],
        ))
        .id()
}

/// (Re)tiles a cable's wire body from fixed-size `CableSegment` copies
/// whenever its endpoints change (initial placement, or an edit-mode drag
/// snapping it to a new cell). Despawns and respawns from scratch each time
/// rather than diffing — simple, and cheap since segment counts stay small
/// (one per grid cell spanned) and this only runs on grid-snap boundaries,
/// not every frame of a drag.
pub fn rebuild_cable_segments(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    changed_cables: Query<(Entity, &Cable, &Children), Changed<Cable>>,
    segments: Query<Entity, With<CableSegment>>,
) {
    for (cable_entity, cable, children) in &changed_cables {
        for &child in children {
            if segments.contains(child) {
                commands.entity(child).despawn();
            }
        }

        let length = (cell_to_world(cable.end) - cell_to_world(cable.start)).length();
        let count = (length / GRID_CELL_SIZE).round().max(1.0) as usize;

        commands.entity(cable_entity).with_children(|parent| {
            for _ in 0..count {
                parent.spawn((
                    CableSegment,
                    Sprite::default(),
                    PendingAppearance(asset_server.load("appearances/cable_center.json")),
                ));
            }
        });
    }
}

/// Drives every cable's wire-body tiles and endpoint caps from its `Cable`
/// endpoints and `SignalValue`: tints all pieces to the current signal
/// color, and positions/rotates each fixed-size `CableSegment` tile along
/// the start-end line (cables aren't constrained to axis-aligned runs, so a
/// per-tile rotation handles any angle) while pinning the two endpoint caps
/// exactly on `start`/`end`.
pub fn sync_cable_sprite(
    cables: Query<(&Cable, &SignalValue, &Children)>,
    mut segments: Query<(&mut Sprite, &mut Transform), (With<CableSegment>, Without<CableEndpoint>)>,
    mut endpoints: Query<
        (&CableEndpoint, &mut Sprite, &mut Transform),
        Without<CableSegment>,
    >,
) {
    for (cable, signal, children) in &cables {
        let color = signal_color(read_logic(signal.0));
        let start = cell_to_world(cable.start);
        let end = cell_to_world(cable.end);
        let delta = end - start;
        let length = delta.length().max(1.0);
        let rotation = Quat::from_rotation_z(delta.y.atan2(delta.x));
        let direction = delta / length;

        let segment_children: Vec<Entity> = children
            .iter()
            .filter(|&child| segments.contains(child))
            .collect();
        let count = segment_children.len().max(1) as f32;
        let segment_length = length / count;

        for (i, &child) in segment_children.iter().enumerate() {
            if let Ok((mut sprite, mut transform)) = segments.get_mut(child) {
                sprite.color = color;
                sprite.custom_size = Some(Vec2::new(segment_length, GRID_CELL_SIZE));
                let center = start + direction * segment_length * (i as f32 + 0.5);
                transform.translation = center.extend(0.0);
                transform.rotation = rotation;
            }
        }

        for &child in children {
            if let Ok((endpoint, mut sprite, mut transform)) = endpoints.get_mut(child) {
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
