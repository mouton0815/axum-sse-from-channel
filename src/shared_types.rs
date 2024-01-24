use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;

/// Message sent through the channel and then to the SSE client
#[derive(Clone, Serialize, Debug)]
pub struct Message {
    text: String
}

impl Message {
    pub fn new(text: String) -> Self {
        Self { text }
    }
}

/// A broadcast message sender shareable between threads
pub type MutexSender = Arc<Mutex<Sender<Message>>>;
