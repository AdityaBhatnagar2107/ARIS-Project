use crate::events::Event;
use crate::types::*;
use tokio::sync::mpsc;
use std::time::Instant;

use tokio::time::{timeout, Duration};
use std::sync::Arc;
use crate::orchestrator::{Task, merge_results};
use crate::network::WorkerPool;

pub async fn run_chaos_test(
    event_tx: mpsc::Sender<Event>,
    interner_id: StringId,
) {
    let start = Instant::now();
    println!("   Starting Chaos Test: 100,000 rapid mutations...");

    // 1. Hammer the Event Pipeline (L4 Stress)
    for i in 0..100_000 {
        let node_id = i as u32;
        // Rapidly add and clear to test bidirectional edge cleanup (L2)
        let _ = event_tx.send(Event::AddNode { id: node_id, kind: NodeType::Function, file_id: interner_id }).await;

        if i % 100 == 0 {
            let _ = event_tx.send(Event::ClearFile { file_id: interner_id }).await;
        }
    }

    // 2. Validate Performance (L10 Constraint)
    let duration = start.elapsed();
    println!("   Chaos Test Complete in {:?}. Throughput: {} ops/sec",
              duration, 100_000 / duration.as_secs().max(1));
}

pub async fn safe_orchestration_loop(
    tasks: Vec<Task>,
    worker_pool: Arc<WorkerPool>
) -> String {
    let mut results = Vec::new();
    let mut loop_count = 0;
    const MAX_REASONING_STEPS: u8 = 5; // Hard invariant

    for task in tasks {
        if loop_count >= MAX_REASONING_STEPS {
            println!("    Loop Guard: Aborting execution to prevent infinite reasoning.");
            break;
        }

        // Enforce 3-second timeout per distributed worker task (L8)
        match timeout(Duration::from_secs(3), worker_pool.execute(task.clone())).await {
            Ok(Ok(result)) => {
                results.push(result.output);
                loop_count += 1;
            },
            Ok(Err(_)) => println!("  Worker Execution Error"),
            Err(_) => println!("   Worker Timeout: Task Rejection triggered"),
        }
    }

    merge_results(results)
}
