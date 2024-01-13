use std::sync::Arc;
use tokio::sync::broadcast::Sender;
use tokio::sync::Mutex;

pub type MutexSender = Arc<Mutex<Sender<String>>>;
