//! Find all cycles in a graph
//!
//! A naive implementation of Johnson's algorithm to find all cycles
//! in a graph. Based on [petgraph](https://github.com/petgraph/petgraph).
//!
//! # Example
//!
//! The triangle graph has exactly one cycle, namely the full graph itself.
//!
//! ```rust
//! use graph_cycles::Cycles;
//! use petgraph::graph::Graph;
//!
//! let g = Graph::<(), ()>::from_edges([(0, 1), (1, 2), (2, 0)]);
//!
//! // find all cycles
//! let cycles = g.cycles();
//! assert_eq!(cycles.len(), 1);
//! assert_eq!(cycles[0].len(), 3);
//!
//! // print each cycle in turn
//! g.visit_all_cycles(|_g, c| {
//!    println!("Found new cycle with vertices {c:?}");
//! });
//! ```
//!
//! # Caveats
//!
//! This crate is essentially untested.
//!
//! # References
//!
//! Donald B. Johnson,
//! Finding all the elementary circuits of a directed graph,
//! SIAM Journal on Computing, 1975.
//!
use std::ops::ControlFlow;

use ahash::AHashSet;
use petgraph::{
    algo::tarjan_scc,
    stable_graph::IndexType,
    visit::{GraphBase, IntoNeighbors, IntoNodeIdentifiers, NodeIndexable},
    EdgeType, Graph,
};

/// Trait for identifying cycles in a graph
pub trait Cycles {
    //! The node identifier of the underlying graph
    type NodeId;

    /// Apply the `visitor` to each cycle until we are told to stop
    ///
    /// The first argument passed to the visitor is a reference to the
    /// graph and the second one a slice with all nodes that form the
    /// cycle. If at any point the visitor returns
    /// `ControlFlow::Break(b)` this function stops visiting any
    /// further cycles and returns `Some(b)`. Otherwise the return
    /// value is `None`.
    fn visit_cycles<F, B>(&self, visitor: F) -> Option<B>
    where
        F: FnMut(&Self, &[Self::NodeId]) -> ControlFlow<B>;

    /// Apply the `visitor` to each cycle until we are told to stop
    ///
    /// The first argument passed to the visitor is a reference to the
    /// graph and the second one a slice with all nodes that form the
    /// cycle.
    fn visit_all_cycles<F>(&self, mut visitor: F)
    where
        F: FnMut(&Self, &[Self::NodeId]),
    {
        self.visit_cycles(|g, n| {
            visitor(g, n);
            ControlFlow::<(), ()>::Continue(())
        });
    }

    /// Find all cycles
    ///
    /// Each element of the returned `Vec` is a `Vec` of all nodes in one cycle.
    fn cycles(&self) -> Vec<Vec<Self::NodeId>>;
}

impl<N, E, Ty: EdgeType, Ix: IndexType> Cycles for Graph<N, E, Ty, Ix> {
    type NodeId = <Graph<N, E, Ty, Ix> as GraphBase>::NodeId;

    fn visit_cycles<F, B>(&self, mut visitor: F) -> Option<B>
    where
        F: FnMut(&Graph<N, E, Ty, Ix>, &[Self::NodeId]) -> ControlFlow<B>,
    {
        for component in tarjan_scc(self) {
            let mut finder = CycleFinder::new(self, component);
            if let ControlFlow::Break(b) = finder.visit(&mut visitor) {
                return Some(b);
            }
        }
        None
    }

    fn cycles(&self) -> Vec<Vec<Self::NodeId>> {
        let mut cycles = Vec::new();
        self.visit_all_cycles(|_, cycle| cycles.push(cycle.to_vec()));
        cycles
    }
}

// // TODO: when trying to use this on a petgraph::graph::Graph rust
// //       complains that `IntoNeighbors` and `IntoNodeIdentifiers` are
// //       not satisfied
// impl<Graph> Cycles for Graph
// where
//     Graph: IntoNodeIdentifiers + IntoNeighbors + NodeIndexable,
// {
//     type NodeId = Graph::NodeId;

//     fn visit_cycles<F, B>(&self, mut visitor: F) -> Option<B>
//     where F: FnMut(&Graph, &[Self::NodeId]) -> ControlFlow<B> {
//         for component in tarjan_scc(self) {
//             let mut finder = CycleFinder::new(self, component);
//             if let ControlFlow::Break(b) = finder.visit(&mut visitor) {
//                 return Some(b);
//             }
//         }
//         None
//     }

//     fn cycles(&self) -> Vec<Vec<Self::NodeId>>  {
//         let mut cycles = Vec::new();
//         self.visit_cycles(|_, cycle| {
//             cycles.push(cycle.to_vec());
//             ControlFlow::<(), ()>::Continue(())
//         });
//         cycles
//     }
// }

#[derive(Clone, Debug, Eq, PartialEq)]
struct CycleFinder<G, N> {
    graph: G,
    scc: Vec<N>,
    blocked: Vec<bool>,
    b: Vec<AHashSet<usize>>,
    stack: Vec<N>,
    s: usize,
}

impl<G> CycleFinder<G, G::NodeId>
where
    G: IntoNodeIdentifiers + IntoNeighbors + NodeIndexable,
{
    fn new(graph: G, scc: Vec<G::NodeId>) -> Self {
        let num_vertices = scc.len();
        Self {
            graph,
            scc,
            blocked: vec![false; num_vertices],
            b: vec![Default::default(); num_vertices],
            stack: Default::default(),
            s: Default::default(),
        }
    }

    fn visit<F, B>(&mut self, visitor: &mut F) -> ControlFlow<B>
    where
        F: FnMut(G, &[G::NodeId]) -> ControlFlow<B>,
    {
        // cycle finding algorithm from
        for s in 0..self.scc.len() {
            self.s = s;
            self.blocked[s..].fill(false);
            for b in &mut self.b[s + 1..] {
                b.clear();
            }
            if let ControlFlow::Break(b) = self.circuit(s, visitor) {
                return ControlFlow::Break(b);
            }
            self.blocked[s] = true;
        }
        ControlFlow::Continue(())
    }

    fn circuit<B, F>(
        &mut self,
        v: usize,
        visitor: &mut F,
    ) -> ControlFlow<B, bool>
    where
        F: FnMut(G, &[G::NodeId]) -> ControlFlow<B>,
    {
        let mut f = false;
        self.stack.push(self.scc[v]);
        self.blocked[v] = true;

        // L1:
        for w in self.adjacent_vertices(v) {
            if w == self.s {
                if let ControlFlow::Break(b) = visitor(self.graph, &self.stack)
                {
                    return ControlFlow::Break(b);
                }
                f = true;
            } else if !self.blocked[w]
                && matches!(
                    self.circuit(w, visitor),
                    ControlFlow::Continue(true)
                )
            {
                f = true;
            }
        }

        // L2:
        if f {
            self.unblock(v)
        } else {
            for w in self.adjacent_vertices(v) {
                self.b[w].insert(v);
            }
        }

        self.stack.pop(); // v
        ControlFlow::Continue(f)
    }

    fn unblock(&mut self, v: usize) {
        self.blocked[v] = false;
        let tmp = self.b[v].clone();
        for w in tmp {
            if self.blocked[w] {
                self.unblock(w)
            }
        }
        self.b[v].clear()
    }

    fn adjacent_vertices(&self, v: usize) -> Vec<usize> {
        self.graph
            .neighbors(self.scc[v])
            .filter_map(|n| self.scc.iter().position(|v| *v == n))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {}
}
