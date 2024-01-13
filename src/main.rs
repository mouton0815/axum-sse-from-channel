use log::info;
use std::sync::Arc;
use tokio::join;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex};
use crate::data_producer::spawn_data_producer;
use crate::http_server::spawn_http_server;
use crate::shutdown_signal::shutdown_signal;
use crate::types::MutexSender;

mod types;
mod data_producer;
mod http_server;
mod shutdown_signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Channel for sending data from the producer to the SSE handler
    let (tx_data, _rx_data) = broadcast::channel::<String>(3);
    let tx_data = Arc::new(Mutex::new(tx_data));

    // Channel for distributing the termination signal to the treads
    let (tx_term, rx_term1) = broadcast::channel(1);
    let rx_term2 = tx_term.subscribe();

    let listener = TcpListener::bind("localhost:3000").await?;
    let handle1 = spawn_http_server(listener, tx_data.clone(), rx_term1);
    let handle2 = spawn_data_producer(tx_data.clone(), rx_term2);

    shutdown_signal().await;
    info!("Termination signal received");
    tx_term.send(())?;

    let (_,_) = join!(handle1, handle2);
    info!("Terminated");
    Ok(())
}
