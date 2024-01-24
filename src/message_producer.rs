use std::time::Duration;
use axum::BoxError;
use log::{debug, info, warn};
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tokio::time;
use crate::Message;
use crate::shared_types::MutexSender;

async fn task(iteration: i64, tx_message: &MutexSender) -> Result<(), BoxError> {
    let msg = Message::new(format!("Foo {}", iteration));
    debug!("Send '{:?}'", msg);
    let guard = tx_message.lock().await;
    (*guard).send(msg)?;
    Ok(())
}

async fn repeat(tx_message: MutexSender, mut rx_shutdown: Receiver<()>) {
    let mut interval = time::interval(Duration::from_secs(1));
    let mut iteration = 0;
    loop {
        tokio::select! {
            _ = interval.tick() => {
                iteration += 1;
                if let Err(e) = task(iteration, &tx_message).await {
                    warn!("Task failed: {:?}, leave producer", e);
                    break;
                }
            },
            _ = rx_shutdown.recv() => {
                info!("Termination signal received, leave producer");
                break;
            }
        }
    }
}

pub fn spawn_message_producer(tx_message: MutexSender, rx_shutdown: Receiver<()>) -> JoinHandle<()> {
    info!("Spawn message producer");
    tokio::spawn(async move {
        repeat(tx_message, rx_shutdown).await;
    })
}