use std::convert::Infallible;
use axum::extract::State;
use axum::response::Sse;
use axum::response::sse::Event;
use axum::Router;
use axum::routing::get;
use axum_macros::debug_handler;
use futures::Stream;
use log::info;
use tokio::net::TcpListener;
use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _; // Enable Iterator trait for BroadcastStream
use crate::MutexSender;

async fn create_stream(tx_data: &MutexSender) -> BroadcastStream<String> {
    let guard = tx_data.lock().await;
    let rx_data = (*guard).subscribe();
    BroadcastStream::new(rx_data)
}

#[debug_handler]
async fn sse_handler(State(tx_data): State<MutexSender>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("Connected");
    let stream = create_stream(&tx_data).await;
    let stream = stream.map(move |item| {
        Ok::<Event, Infallible>(Event::default().data(item.unwrap()))
    });
    Sse::new(stream)
}

pub fn spawn_http_server(listener: TcpListener, tx_data: MutexSender, mut rx_term: Receiver<()>) -> JoinHandle<()> {
    info!("Spawn HTTP server");
    let router = Router::new()
        .route("/sse", get(sse_handler))
        .with_state(tx_data);

    tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                rx_term.recv().await.unwrap();
                info!("Termination signal received, leave HTTP server");
            })
            .await
            .unwrap() // Panic accepted
    })
}
