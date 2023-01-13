# graph-cycles

Find all cycles in a graph

A naive implementation of Johnson's algorithm to find all cycles
in a graph. Based on [petgraph](https://github.com/petgraph/petgraph).

## Example

The triangle graph has exactly one cycle, namely the full graph itself.

```rust
use graph_cycles::Cycles;
use petgraph::graph::Graph;

let g = Graph::<(), ()>::from_edges([(0, 1), (1, 2), (2, 0)]);

// find all cycles
let cycles = g.cycles();
assert_eq!(cycles.len(), 1);
assert_eq!(cycles[0].len(), 3);

// print each cycle in turn
g.visit_all_cycles(|_g, c| {
   println!("Found new cycle with vertices {c:?}");
});
```

## Caveats

This crate is essentially untested.

## References

Donald B. Johnson,
Finding all the elementary circuits of a directed graph,
SIAM Journal on Computing, 1975.


License: MIT or Apache-2.0
