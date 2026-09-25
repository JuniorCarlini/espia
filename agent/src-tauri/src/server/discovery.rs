//! Agent discovery (`protocol/README.md` §1.1-§1.2): mDNS advertisement,
//! with a UDP broadcast responder as the fallback for networks where mDNS
//! doesn't reach (client-isolated Wi-Fi, aggressive IGMP snooping, flaky
//! Windows mDNS). A device tries both — this agent just needs to answer
//! both, not decide which one "wins".

use std::net::SocketAddr;
use std::sync::Arc;

use mdns_sd::{ServiceDaemon, ServiceInfo};
use tokio::net::UdpSocket;

use super::protocol::{Announce, Discover};
use super::SharedState;

const MDNS_SERVICE_TYPE: &str = "_espia._tcp.local.";
const UDP_BIND_ADDR: &str = "0.0.0.0:47800";
const WS_PATH: &str = "/v1/ws";

/// Registers this agent's `_espia._tcp` mDNS service (protocol §1.1) and
/// returns the daemon handle — it must be kept alive for as long as the
/// advertisement should stay up (dropping it unregisters the service), so
/// the caller holds onto it, typically for the process's whole lifetime.
pub fn advertise_mdns(agent_id: &str, agent_name: &str, ws_port: u16) -> Result<ServiceDaemon, String> {
    let daemon = ServiceDaemon::new().map_err(|e| format!("could not start mDNS daemon: {e}"))?;

    // A DNS-safe host label, independent of the (possibly spacey,
    // user-chosen) agent name — see `ServiceInfo::new`'s doc comment on why
    // `host_name` and `my_name` are different parameters.
    let host_name = format!("espia-{agent_id}.local.");
    let properties = [
        ("v", "1"),
        ("id", agent_id),
        ("name", agent_name),
        ("path", WS_PATH),
    ];

    let service = ServiceInfo::new(
        MDNS_SERVICE_TYPE,
        agent_name,
        &host_name,
        "", // let mdns-sd find this machine's real addresses itself
        ws_port,
        &properties[..],
    )
    .map_err(|e| format!("invalid mDNS service info: {e}"))?
    .enable_addr_auto();

    daemon
        .register(service)
        .map_err(|e| format!("could not register mDNS service: {e}"))?;

    eprintln!("espia: advertising {MDNS_SERVICE_TYPE} as {agent_name:?} on port {ws_port}");
    Ok(daemon)
}

/// Answers UDP broadcast `discover` datagrams with a unicast `announce`
/// (protocol §1.2). Runs until the process exits.
pub async fn serve_udp(state: Arc<SharedState>) {
    let socket = match UdpSocket::bind(UDP_BIND_ADDR).await {
        Ok(socket) => socket,
        Err(error) => {
            eprintln!("espia: could not bind UDP discovery responder on {UDP_BIND_ADDR}: {error}");
            return;
        }
    };
    eprintln!("espia: UDP discovery responder listening on {UDP_BIND_ADDR}");

    let mut buf = [0u8; 512];
    loop {
        let (len, src) = match socket.recv_from(&mut buf).await {
            Ok(received) => received,
            Err(error) => {
                eprintln!("espia: UDP recv error: {error}");
                continue;
            }
        };
        handle_datagram(&socket, &buf[..len], src, &state).await;
    }
}

async fn handle_datagram(socket: &UdpSocket, bytes: &[u8], src: SocketAddr, state: &SharedState) {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return;
    };
    let Ok(discover) = serde_json::from_str::<Discover>(text) else {
        return; // not a message we understand — ignore, don't error
    };
    if discover.kind != "discover" {
        return;
    }

    let announce = Announce {
        v: 1,
        kind: "announce",
        agent_id: state.agent_id.clone(),
        name: state.agent_name(),
        port: state.ws_port,
        path: WS_PATH,
    };
    let Ok(json) = serde_json::to_string(&announce) else {
        return;
    };
    if let Err(error) = socket.send_to(json.as_bytes(), src).await {
        eprintln!("espia: could not reply to discover from {src}: {error}");
    }
}
