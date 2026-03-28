mod ai;
pub mod types;
pub mod interner;
pub mod graph;
pub mod parser;
pub mod events;
pub mod traversal;
pub mod context;
pub mod orchestrator;
pub mod network;
pub mod chaos_test;
pub mod github;

use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;
use crate::events::{Event, GraphState, run_event_pipeline};
use crate::interner::Interner;
use crate::traversal::extract_bounded_subgraph;
use crate::context::ContextBuilder;
use crate::github::fetch_github_graph;

fn select_start_nodes(graph: &crate::graph::Graph) -> Vec<crate::types::NodeId> {
    let mut nodes: Vec<crate::types::NodeId> = graph.adj_out.keys().copied().collect();
    if nodes.is_empty() { return vec![]; }

    let mut starts = Vec::new();

    if let Some(&id) = nodes.iter().max_by_key(|&&id| graph.in_degree.get(&id).unwrap_or(&0)) {
        starts.push(id);
    }

    if let Some(&id) = nodes.iter().max_by_key(|&&id| graph.out_degree.get(&id).unwrap_or(&0)) {
        if !starts.contains(&id) { starts.push(id); }
    }

    nodes.sort_by_key(|&id| std::cmp::Reverse(
        graph.in_degree.get(&id).unwrap_or(&0) + graph.out_degree.get(&id).unwrap_or(&0)
    ));

    let top_count = (nodes.len() as f32 * 0.2).max(1.0) as usize;
    let top_nodes = &nodes[..top_count.min(nodes.len())];

    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();

    if let Some(&id) = top_nodes.choose(&mut rng) {
        if !starts.contains(&id) { starts.push(id); }
    }

    starts
}

#[tokio::main]
async fn main() {
    // ✅ LOAD ENV VARIABLES
    dotenv::dotenv().ok();

   
    println!("A.R.I.S. Deterministic Graph Code Intelligence System is Active.");

    let shared_state = Arc::new(RwLock::new(GraphState::new()));
    let interner = Arc::new(RwLock::new(Interner::new()));
    let (_event_tx, event_rx) = mpsc::channel::<Event>(100_000);

    let pipeline_state = shared_state.clone();
    tokio::spawn(async move {
        run_event_pipeline(event_rx, pipeline_state).await;
    });

    let listener = TcpListener::bind("0.0.0.0:9001")
        .await
        .expect("Failed to bind port 9001");

    println!("ARIS backend listening on ws://127.0.0.1:9001");

    while let Ok((stream, _addr)) = listener.accept().await {
        let state_clone = shared_state.clone();
        let interner_clone = interner.clone();

        tokio::spawn(async move {
            let ws_stream = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(_) => return,
            };

            let (mut write, mut read) = ws_stream.split();

            while let Some(msg) = read.next().await {
                let text = match msg {
                    Ok(Message::Text(t)) => t,
                    _ => break,
                };

                let parsed: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let msg_type = parsed.get("type").and_then(|t| t.as_str()).unwrap_or("");

                if msg_type == "load_repo" {
                    let owner = parsed.get("owner").and_then(|t| t.as_str()).unwrap_or("demo").to_string();
                    let repo = parsed.get("repo").and_then(|t| t.as_str()).unwrap_or("aris").to_string();

                    let cache_file = format!("{}_{}_cache.json", owner, repo);
                    let mut loaded_from_cache = false;
                    let mut payload: Option<crate::types::GraphPayload> = None;

                    if std::path::Path::new(&cache_file).exists() {
                        if let Ok(json_str) = std::fs::read_to_string(&cache_file) {
                            if let Ok(p) = serde_json::from_str::<crate::types::GraphPayload>(&json_str) {
                                payload = Some(p);
                                loaded_from_cache = true;
                            }
                        }
                    }

                    let mut gs = state_clone.write().await;
                    let mut int = interner_clone.write().await;

                    gs.graph = crate::graph::Graph::new();
                    *int = Interner::new();

                    if !loaded_from_cache {
                        let _ = fetch_github_graph(&owner, &repo, &mut *int, &mut gs.graph, |_curr, _tot| {}).await;
                    } else if let Some(p) = &payload {
                        for n in &p.nodes {
                            let id = int.intern(&n.label);
                            gs.graph.add_node(id);
                        }
                        for e in &p.edges {
                            let src_label = p.nodes.iter().find(|n| n.id == e.source).map(|n| n.label.as_str());
                            let dst_label = p.nodes.iter().find(|n| n.id == e.target).map(|n| n.label.as_str());
                            if let (Some(src_l), Some(dst_l)) = (src_label, dst_label) {
                                let src = int.intern(src_l);
                                let dst = int.intern(dst_l);
                                gs.graph.add_edge(src, dst, crate::types::EdgeType::Imports);
                            }
                        }
                    }

                    if let Some(p) = payload {
                        let resp = json!({"type": "graph", "nodes": p.nodes, "edges": p.edges}).to_string();
                        let _ = write.send(Message::Text(resp)).await;
                    }

                } else if msg_type == "query" {
                    let question = parsed.get("question").and_then(|q| q.as_str()).unwrap_or("");

                    let gs = state_clone.read().await;
                    let int = interner_clone.read().await;

                    if gs.graph.adj_out.is_empty() { continue; }

                    let starts = select_start_nodes(&gs.graph);

                    let mut merged_nodes = std::collections::HashSet::new();
                    let mut merged_edges = Vec::new();

                    for start_id in starts {
                        let subgraph = extract_bounded_subgraph(&gs.graph, start_id);
                        merged_nodes.extend(subgraph.nodes);
                        merged_edges.extend(subgraph.edges);
                    }

                    let merged_subgraph = crate::traversal::Subgraph {
                        nodes: merged_nodes,
                        edges: merged_edges,
                    };

                    let context = ContextBuilder::new(&*int, 8000)
                        .build(&gs.graph, &merged_subgraph);

                    match ai::ask_gemini(context, question.to_string()).await {
                        Ok(answer) => {
                            let highlighted: Vec<u32> = merged_subgraph.nodes.iter().copied().collect();

                            let resp = json!({
                                "type": "answer",
                                "answer": answer,
                                "highlighted_nodes": highlighted,
                            }).to_string();

                            let _ = write.send(Message::Text(resp)).await;
                        }
                        Err(e) => {
                            let error_payload = json!({
                                "type": "error",
                                "message": e.to_string()
                            }).to_string();

                            let _ = write.send(Message::Text(error_payload)).await;
                        }
                    }
                }
            }
        });
    }
}