use log::{debug, info, warn};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use axum::extract::State;
use axum::response::Sse;
use axum::response::sse::Event;
use axum::{BoxError, Router};
use axum::routing::get;
use axum_macros::debug_handler;
use futures::stream::Stream;
use tokio::{join, signal, time};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex};
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt as _; // Enable Iterator trait for BroadcastStream
use tokio_stream::wrappers::BroadcastStream;

type MutexSender = Arc<Mutex<Sender<String>>>;

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

fn spawn_http_server(listener: TcpListener, tx_data: MutexSender, mut rx_term: Receiver<()>) -> JoinHandle<()> {
    info!("Spawn HTTP server");
    let router = Router::new()
        .route("/sse", get(sse_handler))
        .with_state(tx_data);

    tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                rx_term.recv().await.unwrap();
                info!("Termination signal received, leave HTTP server");
            }).await.unwrap()
    })
}

async fn task(i: i64, tx_data: &MutexSender) -> Result<(), BoxError> {
    let msg = format!("Foo {}", i);
    debug!("Send '{}'", msg);
    let guard = tx_data.lock().await;
    (*guard).send(msg)?;
    Ok(())
}

async fn repeat(tx_data: MutexSender, mut rx_term: Receiver<()>) {
    let mut interval = time::interval(Duration::from_secs(1));
    let mut i = 0;
    loop {
        tokio::select! {
            _ = interval.tick() => {
                i += 1;
                if let Err(e) = task(i, &tx_data).await {
                    warn!("Task failed: {:?}, leave scheduler", e);
                    break;
                }
            },
            _ = rx_term.recv() => {
                info!("Termination signal received, leave scheduler");
                break;
            }
        }
    }
}

fn spawn_scheduler(tx_data: MutexSender, rx_term: Receiver<()>) -> JoinHandle<()> {
    info!("Spawn scheduler");
    tokio::spawn(async move {
        repeat(tx_data, rx_term).await;
    })
}

// See https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
async fn await_shutdown() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
        let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let (tx_data, _rx_data) = broadcast::channel::<String>(5);
    let tx_data = Arc::new(Mutex::new(tx_data));

    let (tx_term, rx_term1) = broadcast::channel(1);
    let rx_term2 = tx_term.subscribe();

    let listener = TcpListener::bind("localhost:3000").await?;
    let handle1 = spawn_http_server(listener, tx_data.clone(), rx_term1);

    let handle2 = spawn_scheduler(tx_data.clone(), rx_term2);

    await_shutdown().await;
    info!("Termination signal received");
    tx_term.send(())?;

    let (_,_) = join!(handle1, handle2);
    info!("Terminated");
    Ok(())
}
