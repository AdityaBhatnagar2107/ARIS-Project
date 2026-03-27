use std::collections::HashSet;
use crate::types::*;
use crate::graph::Graph;
use crate::interner::Interner;
use crate::traversal::Subgraph;

pub struct ContextBuilder<'a> {
    interner: &'a Interner,
    content: String,
    tokens: usize,
    max_tokens: usize,
    accepted_nodes: HashSet<NodeId>,
}

impl<'a> ContextBuilder<'a> {
    pub fn new(interner: &'a Interner, max_tokens: usize) -> Self {
        Self {
            interner,
            content: String::with_capacity(max_tokens * 4), 
            tokens: 0,
            max_tokens,
            accepted_nodes: HashSet::new(),
        }
    }

    fn estimate_tokens(text: &str) -> usize {
        text.len() / 4  
    }

    pub fn add_section(&mut self, title: &str) {
        let text = format!("\n=== {} ===\n", title);
        self.try_push(&text);
    }

    fn try_push(&mut self, text: &str) -> bool {
        let t = Self::estimate_tokens(text);
        if self.tokens + t > self.max_tokens {
            return false;
        }
        self.content.push_str(text);
        self.tokens += t;
        true
    }

    pub fn build(mut self, graph: &Graph, subgraph: &Subgraph) -> String {
        // 1. Rank nodes by degree (hub priority)
        let mut ranked: Vec<NodeId> = subgraph.nodes.iter().copied().collect();
        ranked.sort_by_key(|n| std::cmp::Reverse(*graph.out_degree.get(n).unwrap_or(&0)));

        self.add_section("ENTITIES");
        
        // 2. Insert nodes, resolving strings via Interner
        for node in ranked {
            if let Some(name) = self.interner.resolve(node) {
                let text = format!("- {}\n", name);
                if self.try_push(&text) {
                    self.accepted_nodes.insert(node);
                } else {
                    break; // Token budget exhausted
                }
            }
        }

        self.add_section("RELATIONSHIPS");

        // 3. Filter and type-cast edges to prevent hallucinations
        for &(src, dst, kind) in &subgraph.edges {
            if self.accepted_nodes.contains(&src) && self.accepted_nodes.contains(&dst) {
                let src_name = self.interner.resolve(src).unwrap_or("UNKNOWN");
                let dst_name = self.interner.resolve(dst).unwrap_or("UNKNOWN");
                
                let verb = match kind {
                    EdgeType::Calls => "CALLS",
                    EdgeType::Imports => "IMPORTS",
                    EdgeType::Contains => "CONTAINS",
                    EdgeType::Extends => "EXTENDS",
                };

                let text = format!("{} [{}] {}\n", src_name, verb, dst_name);
                if !self.try_push(&text) {
                    break;  
                }
            }
        }

        self.content
    }
}
