use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};
use crate::types::*;
use crate::graph::Graph;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Event {
    AddNode { id: StringId, kind: NodeType, file_id: StringId },
    AddEdge { src: StringId, dst: StringId, kind: EdgeType },
    ClearFile { file_id: StringId },
}

pub struct GraphState {
    pub graph: Graph,
    pub file_index: HashMap<StringId, Vec<StringId>>,
}

impl GraphState {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            file_index: HashMap::new(),
        }
    }

    pub fn apply_batch(&mut self, events: &HashSet<Event>) {
        // Enforce ordering: Clears must happen before Adds
        for event in events.iter().filter(|e| matches!(e, Event::ClearFile { .. })) {
            self.apply_event(event);
        }
        for event in events.iter().filter(|e| !matches!(e, Event::ClearFile { .. })) {
            self.apply_event(event);
        }
    }

    fn apply_event(&mut self, event: &Event) {
        match event {
            Event::ClearFile { file_id } => {
                if let Some(nodes) = self.file_index.remove(file_id) {
                    for node in nodes {
                        self.graph.remove_node(node);
                    }
                }
            }
            Event::AddNode { id, kind: _, file_id } => {
                self.graph.add_node(*id);
                self.file_index.entry(*file_id).or_default().push(*id);
            }
            Event::AddEdge { src, dst, kind } => {
                self.graph.add_edge(*src, *dst, *kind);
            }
        }
    }
}

pub async fn run_event_pipeline(
    mut rx: mpsc::Receiver<Event>,
    shared_state: Arc<RwLock<GraphState>>,
) {
    let mut flush_interval = interval(Duration::from_secs(2));
    let mut event_buffer: HashSet<Event> = HashSet::new();

    loop {
        tokio::select! {
            Some(event) = rx.recv() => {
                event_buffer.insert(event);
            }
            _ = flush_interval.tick() => {
                if !event_buffer.is_empty() {
                    let mut state = shared_state.write().await;
                    state.apply_batch(&event_buffer);
                    state.graph.validate(); // Safety check post-batch
                    event_buffer.clear();
                }
            }
        }
    }
}
