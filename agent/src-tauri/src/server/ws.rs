//! The WebSocket server devices connect to (`protocol/README.md` §2-§5).
//!
//! Build-step 1 of the networking rollout (see the plan): every `hello` is
//! accepted unconditionally — there is no pairing check yet, that lands in
//! a later step (`pairing_required`/`pair_request`/`paired`). This step
//! only proves the transport, handshake, and live metrics stream work.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::interval;
use tokio_tungstenite::tungstenite::Message;

use super::protocol::{Hello, MetricsMessage, Welcome};
use super::SharedState;

const WS_PATH: &str = "/v1/ws";
// Protocol requires a ping at least every 10s; a couple of seconds of
// margin avoids racing the device's own 30s dead-connection timeout.
const PING_INTERVAL: Duration = Duration::from_secs(8);

pub async fn serve(bind_addr: &str, state: Arc<SharedState>) {
    let listener = match TcpListener::bind(bind_addr).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("espia: could not bind WebSocket server on {bind_addr}: {error}");
            return;
        }
    };
    eprintln!("espia: WebSocket server listening on {bind_addr}{WS_PATH}");

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(accepted) => accepted,
            Err(error) => {
                eprintln!("espia: failed to accept a connection: {error}");
                continue;
            }
        };
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream, peer, state).await {
                eprintln!("espia: connection from {peer} ended: {error}");
            }
        });
    }
}

// The callback's `Err` variant type (`ErrorResponse`, a full HTTP response)
// is tokio-tungstenite's own signature, not something this function controls.
#[allow(clippy::result_large_err)]
async fn handle_connection(stream: TcpStream, peer: SocketAddr, state: Arc<SharedState>) -> Result<(), String> {
    let mut path_ok = false;
    let ws_stream = tokio_tungstenite::accept_hdr_async(stream, |request: &tokio_tungstenite::tungstenite::handshake::server::Request, response| {
        path_ok = request.uri().path() == WS_PATH;
        Ok(response)
    })
    .await
    .map_err(|e| format!("handshake failed: {e}"))?;

    if !path_ok {
        return Err("rejected: wrong WebSocket path".to_string());
    }

    let (mut write, mut read) = ws_stream.split();

    let hello_text = match read.next().await {
        Some(Ok(Message::Text(text))) => text,
        Some(Ok(_)) => return Err("first message was not text".to_string()),
        Some(Err(error)) => return Err(format!("error reading hello: {error}")),
        None => return Err("connection closed before hello".to_string()),
    };
    let hello: Hello = serde_json::from_str(&hello_text).map_err(|e| format!("bad hello: {e}"))?;
    eprintln!("espia: device {} ({}) said hello from {peer}", hello.device_id, hello.board);

    let welcome = Welcome::new(state.agent_id.clone(), state.agent_name());
    let welcome_json = serde_json::to_string(&welcome).map_err(|e| e.to_string())?;
    write.send(Message::Text(welcome_json.into())).await.map_err(|e| e.to_string())?;

    let mut metrics_rx = state.metrics.subscribe();
    let mut ping_ticker = interval(PING_INTERVAL);

    loop {
        tokio::select! {
            changed = metrics_rx.changed() => {
                if changed.is_err() {
                    break; // sender (the collector task) is gone
                }
                let metrics = metrics_rx.borrow_and_update().clone();
                let message = MetricsMessage::new(metrics, now_ms());
                let json = serde_json::to_string(&message).map_err(|e| e.to_string())?;
                if write.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
            _ = ping_ticker.tick() => {
                if write.send(Message::Ping(Vec::new().into())).await.is_err() {
                    break;
                }
            }
            incoming = read.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    // Pongs and anything else are ignored for now — no
                    // inbound message types exist yet beyond `hello`.
                    _ => {}
                }
            }
        }
    }

    eprintln!("espia: device {} disconnected", hello.device_id);
    Ok(())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
