use warp::Filter;

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
    let mut nodes: Vec<_> = graph.adj_out.keys().copied().collect();
    if nodes.is_empty() { return vec![]; }

    let mut starts = Vec::new();

    if let Some(&id) = nodes.iter().max_by_key(|&&id| graph.in_degree.get(&id).unwrap_or(&0)) {
        starts.push(id);
    }

    if let Some(&id) = nodes.iter().max_by_key(|&&id| graph.out_degree.get(&id).unwrap_or(&0)) {
        if !starts.contains(&id) { starts.push(id); }
    }

    starts
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    println!("🚀 A.R.I.S backend starting...");

    let shared_state = Arc::new(RwLock::new(GraphState::new()));
    let interner = Arc::new(RwLock::new(Interner::new()));
    let (_event_tx, event_rx) = mpsc::channel::<Event>(100_000);

    let pipeline_state = shared_state.clone();
    tokio::spawn(async move {
        run_event_pipeline(event_rx, pipeline_state).await;
    });

    use std::env;

    // ===== PORT SETUP =====
    let base_port: u16 = env::var("PORT")
        .unwrap_or("9001".to_string())
        .parse()
        .unwrap();

    let http_port = base_port;
    let ws_port = base_port + 1;

    // ===== HTTP SERVER (FIXES 502) =====
    let health = warp::path::end()
        .map(|| "ARIS backend is running 🚀");

    tokio::spawn(async move {
        warp::serve(health)
            .run(([0, 0, 0, 0], http_port))
            .await;
    });

    // ===== WEBSOCKET SERVER =====
    let ws_addr = format!("0.0.0.0:{}", ws_port);

    let listener = TcpListener::bind(&ws_addr)
        .await
        .expect("❌ Failed to bind WS");

    println!("✅ WebSocket running on ws://{}", ws_addr);

    while let Ok((stream, _)) = listener.accept().await {
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

                    let mut gs = state_clone.write().await;
                    let mut int = interner_clone.write().await;

                    gs.graph = crate::graph::Graph::new();
                    *int = Interner::new();

                    let _ = fetch_github_graph(&owner, &repo, &mut *int, &mut gs.graph, |_c, _t| {}).await;

                    let nodes: Vec<_> = gs.graph.adj_out.keys().map(|&id| {
                        let label = int.resolve(id).unwrap_or("").to_string();
                        json!({"id": id, "label": label})
                    }).collect();

                    let mut edges = Vec::new();
                    for (src, targets) in &gs.graph.adj_out {
                        for dst in targets {
                            edges.push(json!({
                                "source": src,
                                "target": dst
                            }));
                        }
                    }

                    let resp = json!({
                        "type": "graph",
                        "nodes": nodes,
                        "edges": edges
                    }).to_string();

                    let _ = write.send(Message::Text(resp)).await;

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

                    let context = ContextBuilder::new(&*int, 1200)
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
                            let _ = write.send(Message::Text(
                                json!({"type": "error", "message": e.to_string()}).to_string()
                            )).await;
                        }
                    }
                }
            }
        });
    }
}