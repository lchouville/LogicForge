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
/// no need for union-by-rank. `reset` reuses the backing `Vec`'s allocation
/// across calls instead of reallocating it every tick.
#[derive(Default)]
struct Dsu {
    parent: Vec<usize>,
}

impl Dsu {
    fn reset(&mut self, len: usize) {
        self.parent.clear();
        self.parent.extend(0..len);
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

/// The working set `resolve_nets` needs beyond its `nodes`/`cable_pairs`
/// input — grouped so the caller can hold one `Local<ResolveScratch>` and
/// reuse its allocations every tick instead of building fresh `HashMap`s
/// (and a fresh `Dsu`) on every call.
#[derive(Default)]
struct ResolveScratch {
    dsu: Dsu,
    by_pos: HashMap<IVec2, Vec<usize>>,
    nets: HashMap<usize, Vec<usize>>,
}

/// Groups `nodes` into electrical nets — unioning each cable's own two
/// endpoints (electrical continuity of one physical cable, via `cable_pairs`)
/// plus any nodes sharing the same grid `pos` (connect-by-contact) — then
/// resolves each net's value from its `Output` pins with High > Low > Neutral
/// priority (a net with no driving Output naturally resolves to Neutral) and
/// appends the values to write back onto every `Input` pin and `Cable` node
/// into `writes` (cleared first here, not by the caller, so every call
/// starts from a known-empty buffer while still reusing its allocation).
fn resolve_nets(
    nodes: &[NetNode],
    cable_pairs: &[(usize, usize)],
    signal_of: impl Fn(Entity) -> f32,
    scratch: &mut ResolveScratch,
    writes: &mut Vec<(Entity, f32)>,
) {
    writes.clear();

    scratch.dsu.reset(nodes.len());
    for &(start, end) in cable_pairs {
        scratch.dsu.union(start, end);
    }

    scratch.by_pos.clear();
    for (index, node) in nodes.iter().enumerate() {
        scratch.by_pos.entry(node.pos).or_default().push(index);
    }
    for indices in scratch.by_pos.values() {
        for pair in indices.windows(2) {
            scratch.dsu.union(pair[0], pair[1]);
        }
    }

    scratch.nets.clear();
    for index in 0..nodes.len() {
        let root = scratch.dsu.find(index);
        scratch.nets.entry(root).or_default().push(index);
    }

    for members in scratch.nets.values() {
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
}

/// Everything `stage_net_resolution` needs across ticks, held as a single
/// `Local` so its `Vec`s/`HashMap`s (and `resolve_nets`'s own scratch) keep
/// their allocations tick to tick instead of starting from scratch every
/// time — at a 1ms tick that's up to ~1000 reallocations/sec otherwise, for
/// a net graph shape that barely changes from one tick to the next.
#[derive(Default)]
pub(crate) struct NetResolutionScratch {
    nodes: Vec<NetNode>,
    cable_pairs: Vec<(usize, usize)>,
    signal_values: HashMap<Entity, f32>,
    resolve: ResolveScratch,
    writes: Vec<(Entity, f32)>,
}

pub fn stage_net_resolution(
    pins: Query<(Entity, &Pin, &GlobalTransform, &SignalValue)>,
    cables: Query<(Entity, &Cable)>,
    mut buffer: ResMut<SignalWriteBuffer>,
    mut scratch: Local<NetResolutionScratch>,
) {
    scratch.nodes.clear();
    scratch.signal_values.clear();
    for (entity, pin, transform, signal) in &pins {
        scratch.nodes.push(NetNode {
            pos: world_to_cell(transform.translation().truncate()),
            entity,
            kind: NodeKind::Pin(pin.role),
        });
        scratch.signal_values.insert(entity, signal.0);
    }

    scratch.cable_pairs.clear();
    for (entity, cable) in &cables {
        let start_index = scratch.nodes.len();
        scratch.nodes.push(NetNode {
            pos: cable.start,
            entity,
            kind: NodeKind::CableEnd,
        });
        let end_index = scratch.nodes.len();
        scratch.nodes.push(NetNode {
            pos: cable.end,
            entity,
            kind: NodeKind::CableEnd,
        });
        scratch.cable_pairs.push((start_index, end_index));
    }

    // A plain `&mut` (rather than the `Local` smart pointer) so the borrow
    // checker can see `nodes`/`cable_pairs`/`signal_values` and
    // `resolve`/`writes` as disjoint fields below.
    let scratch = &mut *scratch;
    resolve_nets(
        &scratch.nodes,
        &scratch.cable_pairs,
        |entity| scratch.signal_values.get(&entity).copied().unwrap_or(0.0),
        &mut scratch.resolve,
        &mut scratch.writes,
    );
    buffer.0.append(&mut scratch.writes);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pin_node(id: u32, pos: IVec2, role: PinRole) -> NetNode {
        NetNode {
            pos,
            entity: Entity::from_raw_u32(id).unwrap(),
            kind: NodeKind::Pin(role),
        }
    }

    fn cable_end_node(id: u32, pos: IVec2) -> NetNode {
        NetNode {
            pos,
            entity: Entity::from_raw_u32(id).unwrap(),
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

    fn resolve(
        nodes: &[NetNode],
        cable_pairs: &[(usize, usize)],
        signal_of: impl Fn(Entity) -> f32,
    ) -> Vec<(Entity, f32)> {
        let mut scratch = ResolveScratch::default();
        let mut writes = Vec::new();
        resolve_nets(nodes, cable_pairs, signal_of, &mut scratch, &mut writes);
        writes
    }

    #[test]
    fn disjoint_undriven_inputs_force_neutral_independently() {
        let nodes = [
            pin_node(0, IVec2::new(0, 0), PinRole::Input),
            pin_node(1, IVec2::new(5, 5), PinRole::Input),
        ];
        let writes = resolve(&nodes, &[], |_| 0.0);
        assert_eq!(value_of(Entity::from_raw_u32(0).unwrap(), &writes), 0.0);
        assert_eq!(value_of(Entity::from_raw_u32(1).unwrap(), &writes), 0.0);
    }

    #[test]
    fn output_high_drives_input_on_the_same_node() {
        let nodes = [
            pin_node(0, IVec2::new(0, 0), PinRole::Output),
            pin_node(1, IVec2::new(0, 0), PinRole::Input),
        ];
        let writes = resolve(&nodes, &[], |e| {
            if e == Entity::from_raw_u32(0).unwrap() {
                1.0
            } else {
                0.0
            }
        });
        assert_eq!(value_of(Entity::from_raw_u32(1).unwrap(), &writes), 1.0);
    }

    #[test]
    fn conflicting_drivers_resolve_high_over_low() {
        let nodes = [
            pin_node(0, IVec2::new(0, 0), PinRole::Output),
            pin_node(1, IVec2::new(0, 0), PinRole::Output),
            pin_node(2, IVec2::new(0, 0), PinRole::Input),
        ];
        let writes = resolve(&nodes, &[], |e| {
            if e == Entity::from_raw_u32(0).unwrap() {
                1.0
            } else if e == Entity::from_raw_u32(1).unwrap() {
                -1.0
            } else {
                0.0
            }
        });
        assert_eq!(value_of(Entity::from_raw_u32(2).unwrap(), &writes), 1.0);
    }

    #[test]
    fn cable_bridges_two_otherwise_unconnected_pins() {
        let nodes = [
            pin_node(0, IVec2::new(0, 0), PinRole::Output),
            cable_end_node(1, IVec2::new(0, 0)),
            cable_end_node(1, IVec2::new(10, 10)),
            pin_node(2, IVec2::new(10, 10), PinRole::Input),
        ];
        let writes = resolve(&nodes, &[(1, 2)], |e| {
            if e == Entity::from_raw_u32(0).unwrap() {
                1.0
            } else {
                0.0
            }
        });
        assert_eq!(value_of(Entity::from_raw_u32(2).unwrap(), &writes), 1.0);
    }

    #[test]
    fn isolated_cable_resolves_to_neutral() {
        let nodes = [
            cable_end_node(0, IVec2::new(0, 0)),
            cable_end_node(0, IVec2::new(3, 3)),
        ];
        let writes = resolve(&nodes, &[(0, 1)], |_| 0.0);
        assert_eq!(value_of(Entity::from_raw_u32(0).unwrap(), &writes), 0.0);
    }
}
