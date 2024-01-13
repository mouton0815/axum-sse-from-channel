use std::time::Duration;
use axum::BoxError;
use log::{debug, info, warn};
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tokio::time;
use crate::types::MutexSender;

async fn task(iteration: i64, tx_data: &MutexSender) -> Result<(), BoxError> {
    let msg = format!("Foo {}", iteration);
    debug!("Send '{}'", msg);
    let guard = tx_data.lock().await;
    (*guard).send(msg)?;
    Ok(())
}

async fn repeat(tx_data: MutexSender, mut rx_term: Receiver<()>) {
    let mut interval = time::interval(Duration::from_secs(1));
    let mut iteration = 0;
    loop {
        tokio::select! {
            _ = interval.tick() => {
                iteration += 1;
                if let Err(e) = task(iteration, &tx_data).await {
                    warn!("Task failed: {:?}, leave producer", e);
                    break;
                }
            },
            _ = rx_term.recv() => {
                info!("Termination signal received, leave producer");
                break;
            }
        }
    }
}

pub fn spawn_data_producer(tx_data: MutexSender, rx_term: Receiver<()>) -> JoinHandle<()> {
    info!("Spawn data producer");
    tokio::spawn(async move {
        repeat(tx_data, rx_term).await;
    })
}