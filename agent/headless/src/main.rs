//! Headless HTTP server: exposes the same metrics the desktop GUI shows,
//! over a single `GET /metrics` endpoint, for a VPS or a Docker host with no
//! display. See `docs/adr/0013-headless-agent.md`.
//!
//! Deliberately minimal — one thread, no async runtime, no TLS. A required
//! token guards `/metrics`; real network protection (a firewall, or a
//! reverse proxy doing TLS) is the deploying host's job, not this binary's —
//! it must bind on all interfaces for Docker's port mapping to reach it at
//! all, so it cannot protect itself by binding to localhost alone.

use std::env;

use espia_core::collectors::processes::ProcessUsage;
use espia_core::collectors::system::{SystemCollector, SystemMetrics};
use espia_core::security::constant_time_eq;
use serde::Serialize;
use tiny_http::{Header, Method, Response, Server, StatusCode};

mod docker;

const TOKEN_ENV_VAR: &str = "ESPIA_TOKEN";
const BIND_ENV_VAR: &str = "ESPIA_BIND";
const DEFAULT_BIND: &str = "0.0.0.0:8080";
const TOP_PROCESSES_LIMIT: usize = 10;

#[derive(Serialize)]
struct MetricsResponse {
    #[serde(flatten)]
    system: SystemMetrics,
    top_processes: Vec<ProcessUsage>,
}

fn main() {
    let token = env::var(TOKEN_ENV_VAR).ok().filter(|t| !t.is_empty());
    if token.is_none() {
        eprintln!(
            "warning: {TOKEN_ENV_VAR} is not set — every request to /metrics will be \
             rejected (401) until it is. Set it to a long random value, then send that \
             value back as `Authorization: Bearer <token>`."
        );
    }

    let bind = env::var(BIND_ENV_VAR).unwrap_or_else(|_| DEFAULT_BIND.to_string());
    let server = Server::http(&bind).unwrap_or_else(|err| {
        eprintln!("error: could not bind {bind}: {err}");
        std::process::exit(1);
    });
    println!("espia-headless listening on {bind}");
    println!(
        "note: this binds on all interfaces so a Docker port mapping can reach it — put a \
         firewall or reverse proxy in front of it for real network protection."
    );

    let mut collector = SystemCollector::new();

    for request in server.incoming_requests() {
        let (status, body, content_type) = handle(&mut collector, &request, token.as_deref());
        let mut response = Response::from_string(body).with_status_code(status);
        if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()) {
            response = response.with_header(header);
        }
        let _ = request.respond(response);
    }
}

/// Reads the bearer token from the `Authorization` header and compares it to
/// the one this server was started with. No token configured means no
/// request can ever be authorized — fails closed, not open.
fn is_authorized(request: &tiny_http::Request, token: Option<&str>) -> bool {
    let Some(expected) = token else {
        return false;
    };
    request
        .headers()
        .iter()
        .find(|header| header.field.as_str().as_str().eq_ignore_ascii_case("Authorization"))
        .and_then(|header| header.value.as_str().strip_prefix("Bearer "))
        .is_some_and(|got| constant_time_eq(got.as_bytes(), expected.as_bytes()))
}

fn handle(
    collector: &mut SystemCollector,
    request: &tiny_http::Request,
    token: Option<&str>,
) -> (StatusCode, String, &'static str) {
    if *request.method() != Method::Get {
        return (StatusCode(404), not_found(), "application/json");
    }

    match request.url() {
        "/health" => (StatusCode(200), "ok".to_string(), "text/plain"),
        "/metrics" => {
            if !is_authorized(request, token) {
                return (
                    StatusCode(401),
                    r#"{"error":"missing or invalid token"}"#.to_string(),
                    "application/json",
                );
            }
            let system = collector.refresh();
            let top_processes = collector.top_processes(TOP_PROCESSES_LIMIT);
            let body = serde_json::to_string(&MetricsResponse { system, top_processes })
                .unwrap_or_else(|_| r#"{"error":"failed to serialize metrics"}"#.to_string());
            (StatusCode(200), body, "application/json")
        }
        "/containers" => {
            if !is_authorized(request, token) {
                return (
                    StatusCode(401),
                    r#"{"error":"missing or invalid token"}"#.to_string(),
                    "application/json",
                );
            }
            match docker::container_stats() {
                Ok(containers) => (
                    StatusCode(200),
                    serde_json::to_string(&containers)
                        .unwrap_or_else(|_| r#"{"error":"failed to serialize container stats"}"#.to_string()),
                    "application/json",
                ),
                // Most likely cause: /var/run/docker.sock isn't mounted into
                // this container, or this container can't reach it — see
                // docs/adr/0014-docker-container-stats.md.
                Err(err) => (
                    StatusCode(502),
                    serde_json::to_string(&serde_json::json!({ "error": err }))
                        .unwrap_or_else(|_| r#"{"error":"docker API request failed"}"#.to_string()),
                    "application/json",
                ),
            }
        }
        _ => (StatusCode(404), not_found(), "application/json"),
    }
}

fn not_found() -> String {
    r#"{"error":"not found"}"#.to_string()
}
