use std::collections::{BinaryHeap, HashSet};
use std::cmp::Ordering;
use crate::types::*;
use crate::graph::Graph;

pub struct Subgraph {
    pub nodes: HashSet<NodeId>,
    pub edges: Vec<(NodeId, NodeId, EdgeType)>,
}

#[derive(Copy, Clone, PartialEq)]
struct TraversalState {
    node: NodeId,
    depth: u8,
    score: f32,
}

impl Eq for TraversalState {}

impl Ord for TraversalState {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.partial_cmp(&other.score).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for TraversalState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn extract_bounded_subgraph(graph: &Graph, start: NodeId) -> Subgraph {
    let mut visited = HashSet::new();
    let mut queue = BinaryHeap::new();
    
    let initial_score = 1.0;  
    queue.push(TraversalState { node: start, depth: 0, score: initial_score });

    while let Some(state) = queue.pop() {
        if state.depth > 4 { continue; } // Hard limit: Depth <= 4
        if visited.len() >= 150 { break; } // Hard limit: Nodes <= 150

        // Skip if already visited (prevents cyclic infinite loops)
        if !visited.insert(state.node) { continue; }

        // Supernode handling: Add to `visited` so LLM sees it, but DO NOT expand its neighbors
        if graph.is_supernode(state.node) {
            continue;  
        }

        // Expand neighbors
        if let Some(neighbors) = graph.adj_out.get(&state.node) {
            for &(n, _) in neighbors {
                if !visited.contains(&n) {
                    // Proximity scoring: penalize depth to keep context locally dense
                    let n_score = 1.0 / ((state.depth + 1) as f32);
                    queue.push(TraversalState { node: n, depth: state.depth + 1, score: n_score });
                }
            }
        }
    }

    // O(1) Edge Extraction
    let mut edges = Vec::new();
    for &node in &visited {
        if let Some(neighbors) = graph.adj_out.get(&node) {
            for &(n, kind) in neighbors {
                if visited.contains(&n) {
                    edges.push((node, n, kind));
                }
            }
        }
    }

    Subgraph { nodes: visited, edges }
}
