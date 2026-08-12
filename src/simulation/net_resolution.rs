use std::collections::HashMap;

use bevy::prelude::*;

use crate::grid::world_to_cell;

use super::components::{Cable, Pin, PinRole, SignalValue, SignalWriteBuffer};
use super::logic::{LogicState, read_logic};

#[derive(Debug, Clone, Copy)]
enum NodeKind {
    Pin(PinRole),
    CableEnd,
}

struct NetNode {
    pos: IVec2,
    entity: Entity,
    kind: NodeKind,
}

/// Minimal union-find with path compression — plenty at this entity scale,
/// no need for union-by-rank.
struct Dsu {
    parent: Vec<usize>,
}

impl Dsu {
    fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, a: usize, b: usize) {
        let (root_a, root_b) = (self.find(a), self.find(b));
        if root_a != root_b {
            self.parent[root_a] = root_b;
        }
    }
}

/// Groups `nodes` into electrical nets — unioning each cable's own two
/// endpoints (electrical continuity of one physical cable, via `cable_pairs`)
/// plus any nodes sharing the same grid `pos` (connect-by-contact) — then
/// resolves each net's value from its `Output` pins with High > Low > Neutral
/// priority (a net with no driving Output naturally resolves to Neutral) and
/// returns the values to write back onto every `Input` pin and `Cable` node.
fn resolve_nets(
    nodes: &[NetNode],
    cable_pairs: &[(usize, usize)],
    signal_of: impl Fn(Entity) -> f32,
) -> Vec<(Entity, f32)> {
    let mut dsu = Dsu::new(nodes.len());
    for &(start, end) in cable_pairs {
        dsu.union(start, end);
    }

    let mut by_pos: HashMap<IVec2, Vec<usize>> = HashMap::new();
    for (index, node) in nodes.iter().enumerate() {
        by_pos.entry(node.pos).or_default().push(index);
    }
    for indices in by_pos.values() {
        for pair in indices.windows(2) {
            dsu.union(pair[0], pair[1]);
        }
    }

    let mut nets: HashMap<usize, Vec<usize>> = HashMap::new();
    for index in 0..nodes.len() {
        nets.entry(dsu.find(index)).or_default().push(index);
    }

    let mut writes = Vec::new();
    for members in nets.values() {
        let mut has_high = false;
        let mut has_low = false;
        for &index in members {
            if let NodeKind::Pin(PinRole::Output) = nodes[index].kind {
                match read_logic(signal_of(nodes[index].entity)) {
                    LogicState::High => has_high = true,
                    LogicState::Low => has_low = true,
                    LogicState::Neutral => {}
                }
            }
        }
        let resolved = if has_high {
            1.0
        } else if has_low {
            -1.0
        } else {
            0.0
        };

        for &index in members {
            match nodes[index].kind {
                NodeKind::Pin(PinRole::Input) | NodeKind::CableEnd => {
                    writes.push((nodes[index].entity, resolved));
                }
                NodeKind::Pin(PinRole::Output) => {}
            }
        }
    }
    writes
}

pub fn stage_net_resolution(
    pins: Query<(Entity, &Pin, &GlobalTransform, &SignalValue)>,
    cables: Query<(Entity, &Cable)>,
    mut buffer: ResMut<SignalWriteBuffer>,
) {
    let mut nodes = Vec::new();
    let mut signal_values: HashMap<Entity, f32> = HashMap::new();
    for (entity, pin, transform, signal) in &pins {
        nodes.push(NetNode {
            pos: world_to_cell(transform.translation().truncate()),
            entity,
            kind: NodeKind::Pin(pin.role),
        });
        signal_values.insert(entity, signal.0);
    }

    let mut cable_pairs = Vec::new();
    for (entity, cable) in &cables {
        let start_index = nodes.len();
        nodes.push(NetNode {
            pos: cable.start,
            entity,
            kind: NodeKind::CableEnd,
        });
        let end_index = nodes.len();
        nodes.push(NetNode {
            pos: cable.end,
            entity,
            kind: NodeKind::CableEnd,
        });
        cable_pairs.push((start_index, end_index));
    }

    let writes = resolve_nets(&nodes, &cable_pairs, |entity| {
        signal_values.get(&entity).copied().unwrap_or(0.0)
    });
    buffer.0.extend(writes);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin_node(id: u32, pos: IVec2, role: PinRole) -> NetNode {
        NetNode {
            pos,
            entity: Entity::from_raw(id),
            kind: NodeKind::Pin(role),
        }
    }

    fn cable_end_node(id: u32, pos: IVec2) -> NetNode {
        NetNode {
            pos,
            entity: Entity::from_raw(id),
            kind: NodeKind::CableEnd,
        }
    }

    fn value_of(entity: Entity, writes: &[(Entity, f32)]) -> f32 {
        writes
            .iter()
            .find(|(e, _)| *e == entity)
            .map(|(_, value)| *value)
            .unwrap_or(f32::NAN)
    }

    #[test]
    fn disjoint_undriven_inputs_force_neutral_independently() {
        let nodes = [
            pin_node(0, IVec2::new(0, 0), PinRole::Input),
            pin_node(1, IVec2::new(5, 5), PinRole::Input),
        ];
        let writes = resolve_nets(&nodes, &[], |_| 0.0);
        assert_eq!(value_of(Entity::from_raw(0), &writes), 0.0);
        assert_eq!(value_of(Entity::from_raw(1), &writes), 0.0);
    }

    #[test]
    fn output_high_drives_input_on_the_same_node() {
        let nodes = [
            pin_node(0, IVec2::new(0, 0), PinRole::Output),
            pin_node(1, IVec2::new(0, 0), PinRole::Input),
        ];
        let writes = resolve_nets(&nodes, &[], |e| if e == Entity::from_raw(0) { 1.0 } else { 0.0 });
        assert_eq!(value_of(Entity::from_raw(1), &writes), 1.0);
    }

    #[test]
    fn conflicting_drivers_resolve_high_over_low() {
        let nodes = [
            pin_node(0, IVec2::new(0, 0), PinRole::Output),
            pin_node(1, IVec2::new(0, 0), PinRole::Output),
            pin_node(2, IVec2::new(0, 0), PinRole::Input),
        ];
        let writes = resolve_nets(&nodes, &[], |e| {
            if e == Entity::from_raw(0) {
                1.0
            } else if e == Entity::from_raw(1) {
                -1.0
            } else {
                0.0
            }
        });
        assert_eq!(value_of(Entity::from_raw(2), &writes), 1.0);
    }

    #[test]
    fn cable_bridges_two_otherwise_unconnected_pins() {
        let nodes = [
            pin_node(0, IVec2::new(0, 0), PinRole::Output),
            cable_end_node(1, IVec2::new(0, 0)),
            cable_end_node(1, IVec2::new(10, 10)),
            pin_node(2, IVec2::new(10, 10), PinRole::Input),
        ];
        let writes = resolve_nets(&nodes, &[(1, 2)], |e| if e == Entity::from_raw(0) { 1.0 } else { 0.0 });
        assert_eq!(value_of(Entity::from_raw(2), &writes), 1.0);
    }

    #[test]
    fn isolated_cable_resolves_to_neutral() {
        let nodes = [
            cable_end_node(0, IVec2::new(0, 0)),
            cable_end_node(0, IVec2::new(3, 3)),
        ];
        let writes = resolve_nets(&nodes, &[(0, 1)], |_| 0.0);
        assert_eq!(value_of(Entity::from_raw(0), &writes), 0.0);
    }
}
