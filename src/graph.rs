use std::collections::{HashMap, HashSet};
use crate::types::*;

pub struct Graph {
    pub adj_out: HashMap<NodeId, HashSet<(NodeId, EdgeType)>>,
    pub adj_in: HashMap<NodeId, HashSet<(NodeId, EdgeType)>>,
    pub out_degree: HashMap<NodeId, usize>,
    pub in_degree: HashMap<NodeId, usize>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            adj_out: HashMap::new(),
            adj_in: HashMap::new(),
            out_degree: HashMap::new(),
            in_degree: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, id: NodeId) {
        self.adj_out.entry(id).or_default();
        self.adj_in.entry(id).or_default();
        self.out_degree.entry(id).or_insert(0);
        self.in_degree.entry(id).or_insert(0);
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId, kind: EdgeType) {
        let out_edges = self.adj_out.entry(from).or_default();
        
        if out_edges.insert((to, kind)) {
            self.adj_in.entry(to).or_default().insert((from, kind));

            *self.out_degree.entry(from).or_insert(0) += 1;
            *self.in_degree.entry(to).or_insert(0) += 1;
        }
    }

    pub fn is_supernode(&self, node: NodeId) -> bool {
        let out_deg = self.out_degree.get(&node).unwrap_or(&0);
        let in_deg = self.in_degree.get(&node).unwrap_or(&0);
        *out_deg > 300 || *in_deg > 300
    }

    pub fn remove_node(&mut self, node: NodeId) {
        if let Some(neighbors) = self.adj_out.remove(&node) {
            for (target, kind) in neighbors {
                if let Some(in_edges) = self.adj_in.get_mut(&target) {
                    in_edges.remove(&(node, kind));
                    if let Some(deg) = self.in_degree.get_mut(&target) {
                        *deg = deg.saturating_sub(1);
                    }
                }
            }
        }

        if let Some(neighbors) = self.adj_in.remove(&node) {
            for (source, kind) in neighbors {
                if let Some(out_edges) = self.adj_out.get_mut(&source) {
                    out_edges.remove(&(node, kind));
                    if let Some(deg) = self.out_degree.get_mut(&source) {
                        *deg = deg.saturating_sub(1);
                    }
                }
            }
        }

        self.out_degree.remove(&node);
        self.in_degree.remove(&node);
    }

    pub fn validate(&self) {
        for (node, neighbors) in &self.adj_out {
            for &(n, kind) in neighbors {
                let back = self.adj_in.get(&n).expect("Broken edge consistency");
                assert!(back.contains(&(*node, kind)), "Broken edge consistency");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_edge() {
        let mut g = Graph::new();
        g.add_node(1);
        g.add_node(2);
        g.add_edge(1, 2, EdgeType::Calls);

        assert!(g.adj_out.get(&1).unwrap().contains(&(2, EdgeType::Calls)));
        assert!(g.adj_in.get(&2).unwrap().contains(&(1, EdgeType::Calls)));
    }

    #[test]
    fn test_no_duplicate_edges() {
        let mut g = Graph::new();
        g.add_edge(1, 2, EdgeType::Calls);
        g.add_edge(1, 2, EdgeType::Calls);
        assert_eq!(g.adj_out.get(&1).unwrap().len(), 1);
    }

    #[test]
    fn test_remove_node() {
        let mut g = Graph::new();
        g.add_edge(1, 2, EdgeType::Calls);
        g.remove_node(1);

        assert!(g.adj_out.get(&1).is_none());
        assert!(!g.adj_in.get(&2).unwrap().contains(&(1, EdgeType::Calls)));
    }
}
