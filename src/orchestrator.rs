use std::sync::atomic::{AtomicU32, Ordering};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use futures_util::stream::{FuturesUnordered, StreamExt};

static TASK_COUNTER: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone)]
pub struct Task {
    pub id: u32,
    pub query: String,
    pub context: String, 
}

#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: u32,
    pub output: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum QueryType {
    Local,
    PathTrace,
    GlobalSummary,
}

pub fn classify(query: &str) -> QueryType {
    let q = query.to_lowercase();
    if q.contains("trace") || q.contains("flow") {
        QueryType::PathTrace
    } else if q.contains("summarize") || q.contains("architecture") {
        QueryType::GlobalSummary
    } else {
        QueryType::Local
    }
}

pub struct Orchestrator {
    task_queue: mpsc::Sender<Task>,
}

impl Orchestrator {
    pub fn new(_worker_count: usize) -> (Self, mpsc::Receiver<Task>) {
        let (tx, rx) = mpsc::channel(100);
        (Self { task_queue: tx }, rx)
    }

    pub async fn dispatch(&self, query: String, contexts: Vec<String>) {
        for ctx in contexts {
            let task = Task {
                id: TASK_COUNTER.fetch_add(1, Ordering::SeqCst),
                query: query.clone(),
                context: ctx,
            };
            
            // Non-blocking dispatch
            if self.task_queue.send(task).await.is_err() {
                eprintln!("Worker pool disconnected.");
                break;
            }
        }
    }
}

pub async fn run_worker_pool(
    mut task_rx: mpsc::Receiver<Task>,  
    concurrency_limit: usize
) -> String {
    let mut executing = FuturesUnordered::new();
    let mut final_results = Vec::new();

    loop {
        tokio::select! {
            Some(task) = task_rx.recv(), if executing.len() < concurrency_limit => {
                // Spawn async inference task
                let handle: JoinHandle<TaskResult> = tokio::spawn(async move {
                    execute_llm_inference(task).await
                });
                executing.push(handle);
            }
            
            Some(result) = executing.next() => {
                match result {
                    Ok(res) => final_results.push(res.output),
                    Err(_) => eprintln!("Task panicked"),
                }
            }
            
            else => {
                // Queue is empty and all tasks are finished
                if executing.is_empty() {
                    break;
                }
            }
        }
    }

    merge_results(final_results)
}

// Mock LLM Interface
async fn execute_llm_inference(task: Task) -> TaskResult {
    // Simulate network latency / llama.cpp execution
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    TaskResult {
        task_id: task.id,
        output: format!("[Task {} Result]: Analyzed path for '{}'", task.id, task.query),
    }
}

pub fn merge_results(results: Vec<String>) -> String {
    results.join("\n---\n")
}
