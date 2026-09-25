//! espia-socket-guard: the only thing that ever talks to the real Docker
//! socket. Everything else — `espia-headless` included — talks to this
//! instead, over plain HTTP, and this forwards exactly two request shapes:
//!
//!   GET /containers/json
//!   GET /containers/<id>/stats
//!
//! Anything else — any other path, any non-GET method — is refused with a
//! `403` before it ever reaches the socket. There is no code path from an
//! incoming request to a write call on the socket; there's no code here
//! that *makes* a write call at all.
//!
//! Meant to run as its own container, with `/var/run/docker.sock` mounted
//! into *it* and nothing else — `espia-headless` (or anything else that
//! wants container stats) reaches it over the network instead of the
//! socket. See `docs/adr/0014-docker-container-stats.md`.

use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use tiny_http::{Header, Method, Response, Server, StatusCode};

const SOCKET_ENV_VAR: &str = "DOCKER_SOCKET";
const DEFAULT_SOCKET: &str = "/var/run/docker.sock";
const BIND_ENV_VAR: &str = "BIND";
const DEFAULT_BIND: &str = "0.0.0.0:2375";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

fn main() {
    let bind = env::var(BIND_ENV_VAR).unwrap_or_else(|_| DEFAULT_BIND.to_string());
    let socket = env::var(SOCKET_ENV_VAR).unwrap_or_else(|_| DEFAULT_SOCKET.to_string());

    let server = Server::http(&bind).unwrap_or_else(|err| {
        eprintln!("error: could not bind {bind}: {err}");
        std::process::exit(1);
    });
    println!("espia-socket-guard listening on {bind}, forwarding to {socket}");
    println!(
        "allowlist: GET /containers/json, GET /containers/<id>/stats — everything else is \
         refused before it reaches the socket."
    );

    for request in server.incoming_requests() {
        let (status, body) = handle(&socket, &request);
        let mut response = Response::from_string(body).with_status_code(status);
        if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
            response = response.with_header(header);
        }
        let _ = request.respond(response);
    }
}

fn handle(socket: &str, request: &tiny_http::Request) -> (StatusCode, String) {
    if *request.method() != Method::Get {
        return (StatusCode(403), forbidden());
    }
    let Some(allowed_path) = allowlisted_path(request.url()) else {
        return (StatusCode(403), forbidden());
    };
    match docker_get(socket, &allowed_path) {
        Ok(body) => (StatusCode(200), body),
        Err(err) => (StatusCode(502), format!(r#"{{"error":{err:?}}}"#)),
    }
}

fn forbidden() -> String {
    r#"{"error":"not on the allowlist: only GET /containers/json and GET /containers/<id>/stats are forwarded"}"#
        .to_string()
}

/// The entire security boundary lives here. Returns the exact path to send
/// to the real socket — re-derived from the parsed container id rather
/// than the incoming path passed straight through, so nothing beyond an id
/// can ever ride along (no extra segments, no unexpected query params).
fn allowlisted_path(requested: &str) -> Option<String> {
    let path = requested.split('?').next().unwrap_or(requested);

    if path == "/containers/json" {
        return Some("/containers/json".to_string());
    }

    let id = path.strip_prefix("/containers/")?.strip_suffix("/stats")?;
    let is_safe_id = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    is_safe_id.then(|| format!("/containers/{id}/stats?stream=false"))
}

fn docker_get(socket_path: &str, path: &str) -> Result<String, String> {
    let mut stream = UnixStream::connect(socket_path).map_err(|e| format!("connecting to {socket_path}: {e}"))?;
    stream.set_read_timeout(Some(REQUEST_TIMEOUT)).ok();
    stream.set_write_timeout(Some(REQUEST_TIMEOUT)).ok();

    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).map_err(|e| format!("writing request: {e}"))?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).map_err(|e| format!("reading response: {e}"))?;

    let header_end = find(&raw, b"\r\n\r\n").ok_or("malformed HTTP response from the Docker socket")?;
    let header_text = String::from_utf8_lossy(&raw[..header_end]);
    let body = &raw[header_end + 4..];

    let status_line = header_text.lines().next().unwrap_or("");
    if !status_line.contains(" 200 ") {
        return Err(format!("Docker API returned {status_line:?} for {path}"));
    }

    let body = if header_text.to_ascii_lowercase().contains("transfer-encoding: chunked") {
        dechunk(body)
    } else {
        body.to_vec()
    };
    String::from_utf8(body).map_err(|e| format!("non-UTF8 response body: {e}"))
}

/// Decodes an HTTP chunked-transfer body. The Docker daemon (a Go `net/http`
/// server) uses chunked encoding for these endpoints when it doesn't know
/// the body length up front, which is the common case here.
fn dechunk(mut data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    while let Some(line_end) = find(data, b"\r\n") {
        let size_line = String::from_utf8_lossy(&data[..line_end]);
        let Ok(size) = usize::from_str_radix(size_line.trim(), 16) else { break };
        if size == 0 {
            break;
        }
        let chunk_start = line_end + 2;
        let chunk_end = chunk_start + size;
        if chunk_end > data.len() {
            break;
        }
        out.extend_from_slice(&data[chunk_start..chunk_end]);
        data = data.get(chunk_end + 2..).unwrap_or(&[]);
    }
    out
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_the_container_list() {
        assert_eq!(allowlisted_path("/containers/json"), Some("/containers/json".to_string()));
    }

    #[test]
    fn allows_stats_for_a_specific_container_and_normalizes_the_query() {
        assert_eq!(
            allowlisted_path("/containers/40a69d0bf4be/stats?stream=true"),
            Some("/containers/40a69d0bf4be/stats?stream=false".to_string())
        );
        assert_eq!(
            allowlisted_path("/containers/my-app_1/stats"),
            Some("/containers/my-app_1/stats?stream=false".to_string())
        );
    }

    #[test]
    fn rejects_write_endpoints() {
        assert_eq!(allowlisted_path("/containers/create"), None);
        assert_eq!(allowlisted_path("/containers/40a69d0bf4be/stop"), None);
        assert_eq!(allowlisted_path("/containers/40a69d0bf4be/exec"), None);
    }

    #[test]
    fn rejects_anything_outside_containers() {
        assert_eq!(allowlisted_path("/images/json"), None);
        assert_eq!(allowlisted_path("/version"), None);
        assert_eq!(allowlisted_path("/"), None);
    }

    #[test]
    fn rejects_path_traversal_and_unsafe_ids() {
        assert_eq!(allowlisted_path("/containers/../../etc/passwd/stats"), None);
        assert_eq!(allowlisted_path("/containers//stats"), None);
        assert_eq!(allowlisted_path("/containers/foo bar/stats"), None);
    }

    #[test]
    fn dechunk_reassembles_a_chunked_body() {
        let chunked = b"7\r\nMozilla\r\n9\r\nDeveloper\r\n0\r\n\r\n";
        assert_eq!(dechunk(chunked), b"MozillaDeveloper");
    }
}
