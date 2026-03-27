use serde::{Serialize, Deserialize};

pub type NodeId = u32;
pub type StringId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeType {
    File,
    Function,
    Class,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EdgeType {
    Calls,
    Imports,
    Contains,
    Extends,
}

#[derive(Serialize, Deserialize)]
pub struct GraphPayload {
    pub nodes: Vec<NodePayload>,
    pub edges: Vec<EdgePayload>,
}

#[derive(Serialize, Deserialize)]
pub struct NodePayload {
    pub id: u32,
    pub label: String,
}

#[derive(Serialize, Deserialize)]
pub struct EdgePayload {
    pub source: u32,
    pub target: u32,
    pub kind: String,
}
