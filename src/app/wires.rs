// SPDX-License-Identifier: GPL-2.0-or-later
// Derived from CircuitJS1 (C) Paul Falstad and Iain Sharp (WireConverter).
//! Tools > Convert Wires to Routed Wires: chains of plain wires become routed wires.
//!
//! Two wires that meet at a point are joined, but where three or more meet (a junction), or
//! where the post of another element touches, the chain ends and the wires stay separate.

use super::session::Session;
use crate::elements::geometry::Point;
use crate::elements::passive::WireElm;
use std::collections::{HashMap, HashSet};

/// A wire as its two end points.
pub type Segment = ((i32, i32), (i32, i32));

/// A chain: the indices of its wires and its points from one end to the other.
pub type Chain = (Vec<usize>, Vec<(i32, i32)>);

/// Groups `wires` into chains. `others` are the posts of everything that is not one of the
/// wires. Each chain is the indices of its wires and the points from one end to the other; a
/// chain that closes on itself is left out, as in the original.
pub fn chains(wires: &[Segment], others: &[(i32, i32)]) -> Vec<Chain> {
    let mut count: HashMap<(i32, i32), u32> = HashMap::new();
    let mut at: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (i, (a, b)) in wires.iter().enumerate() {
        for p in [a, b] {
            *count.entry(*p).or_default() += 1;
            at.entry(*p).or_default().push(i);
        }
    }
    // a post of another element on a point makes it an end: add two so it can never be 2
    for p in others {
        if let Some(c) = count.get_mut(p) {
            *c += 2;
        }
    }
    let interior = |p: &(i32, i32)| count.get(p) == Some(&2);
    let mut visited = HashSet::new();
    let mut out = Vec::new();
    for start in 0..wires.len() {
        if visited.contains(&start) {
            continue;
        }
        // every wire reachable through interior points
        let mut chain = Vec::new();
        let mut stack = vec![start];
        while let Some(w) = stack.pop() {
            if !visited.insert(w) {
                continue;
            }
            chain.push(w);
            for p in [wires[w].0, wires[w].1] {
                if interior(&p) {
                    stack.extend(at[&p].iter().filter(|n| !visited.contains(n)));
                }
            }
        }
        if chain.len() == 1 {
            let (a, b) = wires[chain[0]];
            out.push((chain, vec![a, b]));
            continue;
        }
        // walk from an end that is not an interior point
        let in_chain: HashSet<usize> = chain.iter().copied().collect();
        let begin = chain.iter().find_map(|&w| {
            let (a, b) = wires[w];
            if !interior(&a) {
                Some((w, a))
            } else if !interior(&b) {
                Some((w, b))
            } else {
                None
            }
        });
        let Some((mut current, mut point)) = begin else { continue };
        let mut points = vec![point];
        let mut used = HashSet::new();
        loop {
            used.insert(current);
            let (a, b) = wires[current];
            let other = if a == point { b } else { a };
            points.push(other);
            let next = interior(&other)
                .then(|| at[&other].iter().find(|n| in_chain.contains(n) && !used.contains(*n)))
                .flatten();
            match next {
                Some(&n) => {
                    current = n;
                    point = other;
                }
                None => break,
            }
        }
        if points.len() >= 3 {
            out.push((chain, points));
        }
    }
    out
}

impl Session {
    /// Turns the plain wires (the selected ones, when any wire is selected) into routed wires.
    /// Returns how many routed wires were made.
    pub fn convert_wires(&mut self) -> usize {
        if !self.editing_allowed() {
            return 0;
        }
        let plain = |e: &dyn crate::elements::element::Element| {
            e.as_any().downcast_ref::<WireElm>().is_some_and(WireElm::is_plain)
        };
        let any_selected = self.sim.elements.iter().any(|e| plain(e.as_ref()) && e.base().selected);
        let mut index = Vec::new();
        let mut segments = Vec::new();
        let mut others = Vec::new();
        for (i, e) in self.sim.elements.iter().enumerate() {
            if plain(e.as_ref()) && (!any_selected || e.base().selected) {
                let b = e.base();
                index.push(i);
                segments.push(((b.x, b.y), (b.x2, b.y2)));
            } else if e.as_any().downcast_ref::<WireElm>().is_none() {
                for n in 0..e.post_count() {
                    if let Some(p) = e.get_post(n) {
                        others.push((p.x, p.y));
                    }
                }
            }
        }
        let found = chains(&segments, &others);
        if found.is_empty() {
            return 0;
        }
        self.push_undo();
        let mut remove = Vec::new();
        for (wires, points) in &found {
            remove.extend(wires.iter().map(|&w| index[w]));
            let route = points.iter().map(|&(x, y)| Point::new(x, y)).collect();
            self.sim.elements.push(Box::new(WireElm::from_route(route)));
        }
        let made = found.len();
        // the new wires sit after the old ones, so removing the old ones keeps their order
        self.remove_elements(&remove);
        self.dirty = true;
        self.reanalyze();
        made
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(a: (i32, i32), b: (i32, i32)) -> Segment {
        (a, b)
    }

    #[test]
    fn a_run_of_wires_becomes_one_chain_in_order() {
        let wires = [w((16, 0), (16, 16)), w((0, 0), (16, 0)), w((16, 16), (32, 16))];
        let c = chains(&wires, &[]);
        assert_eq!(c.len(), 1);
        let pts = &c[0].1;
        assert_eq!(pts.len(), 4);
        assert!(
            pts == &[(0, 0), (16, 0), (16, 16), (32, 16)]
                || pts == &[(32, 16), (16, 16), (16, 0), (0, 0)]
        );
    }

    #[test]
    fn a_junction_or_a_post_ends_the_chain() {
        // three wires meet at (16, 0)
        let wires = [w((0, 0), (16, 0)), w((16, 0), (32, 0)), w((16, 0), (16, 16))];
        assert_eq!(chains(&wires, &[]).len(), 3);
        // a post of something else in the middle of a run
        let run = [w((0, 0), (16, 0)), w((16, 0), (32, 0))];
        assert_eq!(chains(&run, &[(16, 0)]).len(), 2);
        assert_eq!(chains(&run, &[]).len(), 1);
    }

    #[test]
    fn a_closed_loop_is_left_alone() {
        let ring = [w((0, 0), (16, 0)), w((16, 0), (16, 16)), w((16, 16), (0, 0))];
        assert!(chains(&ring, &[]).is_empty());
    }
}
