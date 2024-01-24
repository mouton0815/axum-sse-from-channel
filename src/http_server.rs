use axum::extract::State;
use axum::response::Sse;
use axum::response::sse::Event;
use axum::{Error, Router};
use axum::routing::get;
use futures::Stream;
use log::info;
use tokio::net::TcpListener;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt; // Enable Iterator trait for BroadcastStream
use crate::{Message, MutexSender};

async fn create_stream(tx_message: &MutexSender) -> BroadcastStream<Message> {
    let guard = tx_message.lock().await;
    let rx_message = (*guard).subscribe();
    BroadcastStream::new(rx_message)
}

async fn sse_handler(State(tx_message): State<MutexSender>) -> Sse<impl Stream<Item = Result<Event, Error>>> {
    info!("Connected");
    let stream = create_stream(&tx_message).await;
    let stream = stream.map(move |item| {
        Event::default().json_data(item.unwrap())
    });
    Sse::new(stream)
}

pub fn spawn_http_server(listener: TcpListener, tx_message: MutexSender, mut rx_shutdown: Receiver<()>) -> JoinHandle<()> {
    info!("Spawn HTTP server");
    let router = Router::new()
        .route("/sse", get(sse_handler))
        .with_state(tx_message);

    tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                rx_shutdown.recv().await.unwrap();
                info!("Termination signal received, leave HTTP server");
            })
            .await
            .unwrap() // Panic accepted
    })
}
