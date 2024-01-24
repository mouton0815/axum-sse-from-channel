use log::info;
use std::sync::Arc;
use tokio::join;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex};
use crate::http_server::spawn_http_server;
use crate::message_producer::spawn_message_producer;
use crate::shutdown_signal::shutdown_signal;
use crate::shared_types::{Message, MutexSender};

mod shared_types;
mod message_producer;
mod http_server;
mod shutdown_signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Channel for sending messages from the producer thread to the axum SSE handler
    let (tx_message, _rx_message) = broadcast::channel::<Message>(1);
    let tx_message = Arc::new(Mutex::new(tx_message));

    // Channel for distributing the termination signal to the threads
    let (tx_shutdown, rx_shutdown1) = broadcast::channel(1);
    let rx_shutdown2 = tx_shutdown.subscribe();

    let listener = TcpListener::bind("localhost:3000").await?;
    let handle1 = spawn_http_server(listener, tx_message.clone(), rx_shutdown1);
    let handle2 = spawn_message_producer(tx_message, rx_shutdown2);

    shutdown_signal().await;
    info!("Termination signal received");
    tx_shutdown.send(())?;

    let (_,_) = join!(handle1, handle2);
    info!("Terminated");
    Ok(())
}
