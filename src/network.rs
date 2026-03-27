use tokio::sync::{mpsc, oneshot, RwLock};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskMessage {
    pub id: u32,
    pub query: String,
    pub context: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResultMessage {
    pub task_id: u32,
    pub output: String,
}

// Internal Task Routing
pub struct PendingTask {
    pub response_tx: oneshot::Sender<ResultMessage>,
    pub timestamp: std::time::Instant,
}

pub struct WorkerPool {
    // Map of TaskID -> Channel to resolve the original request
    pub pending: Arc<RwLock<HashMap<u32, PendingTask>>>,
    // Channel to send tasks to the network dispatcher
    pub task_tx: mpsc::Sender<TaskMessage>,
}

impl WorkerPool {
    pub async fn execute(&self, task: crate::orchestrator::Task) -> Result<ResultMessage, ()> {
        let (tx, rx) = oneshot::channel();
        self.pending.write().await.insert(
            task.id,
            PendingTask {
                response_tx: tx,
                timestamp: std::time::Instant::now(),
            },
        );

        let msg = TaskMessage {
            id: task.id,
            query: task.query,
            context: task.context,
        };

        if self.task_tx.send(msg).await.is_err() {
            return Err(());
        }

        rx.await.map_err(|_| ())
    }
}

// The Non-Blocking Dispatcher (Async)
pub async fn handle_worker_stream(
    ws: WebSocketStream<TcpStream>,
    pending_map: Arc<RwLock<HashMap<u32, PendingTask>>>,
    mut task_rx: mpsc::Receiver<TaskMessage>
) {
    let (mut ws_tx, mut ws_rx) = ws.split();

    loop {
        tokio::select! {
            // A. Send tasks from the Host to the Worker
            Some(task) = task_rx.recv() => {
                let serialized = bincode::serialize(&task).unwrap();
                ws_tx.send(Message::Binary(serialized)).await.unwrap();
            }

            // B. Receive results from the Worker
            Some(Ok(Message::Binary(bin))) = ws_rx.next() => {
                let result: ResultMessage = bincode::deserialize(&bin).unwrap();
                
                // Find the original requester and send them the answer
                if let Some(pending) = pending_map.write().await.remove(&result.task_id) {
                    let _ = pending.response_tx.send(result);
                }
            }
        }
    }
}
