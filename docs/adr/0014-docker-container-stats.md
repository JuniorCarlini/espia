# 0014. Per-container stats via the Docker socket

- **Status:** Accepted
- **Date:** 2026-09-25

## Context

Running `espia-headless` in a container ([0013](0013-headless-agent.md))
made `top_processes` in `GET /metrics` nearly useless: Docker gives every
container its own PID namespace by default, so `sysinfo` — reading
`/proc` from inside the container — only ever sees the agent's own
processes, never the host's real workload. CPU/memory/disk/network
*totals* stay accurate (`/proc/meminfo` and `/proc/stat` aren't
namespaced the same way), only the per-process breakdown is affected.

What was actually wanted wasn't host processes anyway — it was "which
**container** on this VPS is using the most CPU/memory," to compare
`espia`'s own footprint against everything else running there. That's a
different question, answerable only through the Docker Engine API, not
through anything under `/proc`.

## Decision

- **Add `GET /containers`**, gated by the same `ESPIA_TOKEN` as
  `/metrics`. It lists every running container with `cpu_pct`,
  `memory_bytes`, and `memory_limit_bytes`, sorted by CPU descending —
  the same shape and idea as `top_processes`, one level up.
- **Talk to the Docker Engine API directly over its Unix socket**
  (`/var/run/docker.sock` by default, `ESPIA_DOCKER_SOCKET` to override),
  with a small hand-rolled HTTP/1.1 client (raw socket, chunked-encoding
  support) rather than a client crate — every maintained one either pulls
  in an async runtime (`bollard`, on tokio) or is unmaintained, and this
  binary's whole point is staying free of both ([0013](0013-headless-agent.md)).
  CPU % is computed the same way `docker stats` computes it (CPU-time
  delta over system-CPU-time delta, scaled by core count).
- **This requires mounting `/var/run/docker.sock` into the container**,
  and the image now runs as root rather than a dedicated user. Neither is
  optional once this feature is on: **anyone who can reach that socket
  already has root-equivalent control of the entire Docker host** — they
  can start a privileged container and walk straight onto the host
  filesystem. A non-root `USER` in this image would not have changed
  that; it would only have hidden how much access `/containers` actually
  implies.
- **The client supports an alternative, safer target**: `ESPIA_DOCKER_HOST`
  points it at a `docker-socket-proxy` sidecar over plain HTTP instead of
  the real socket (`ESPIA_DOCKER_SOCKET`, still the default). The proxy
  holds the real socket and only forwards the specific calls `/containers`
  needs, refusing everything else — `create`, `stop`, `exec`, and the rest
  come back `403` even if this binary is compromised. This is the
  recommended way to run it; see the headless README.

## Consequences

- `/containers` only exists, and only makes sense, for the headless/Docker
  deployment — there's no equivalent for the desktop GUI, and none is
  planned; a desktop machine isn't a Docker host being compared against
  its own containers.
- Deploying with Docker-socket access is now a deliberate, opt-in step
  (mount the socket, nothing more) — leaving it unmounted keeps the agent
  at exactly [0013](0013-headless-agent.md)'s original blast radius, and
  `/containers` fails closed with a clear `502` instead of the process
  crashing or hanging.
- This is a materially bigger trust boundary than `/metrics` ever was.
  It belongs on a VPS the operator already fully controls (the common
  case here), not on shared or multi-tenant Docker infrastructure where
  "one container reads another's stats" is closer to "one container
  controls every other container."
- Verified against a real Docker daemon: container list, computed CPU %,
  and memory usage were cross-checked against `docker stats` itself on
  the same host and matched within normal sampling variance. Separately
  verified the `docker-socket-proxy` path end to end, including that its
  `403` on write calls actually holds.

## Alternatives considered

- **`--pid=host`** (share the host's PID namespace instead) — considered
  first, but only surfaces raw host processes, not grouped by container,
  which wasn't the actual question ("which container is heaviest,"
  not "which process is heaviest").
- **A Docker client crate** (`bollard`) — the natural reach for a real
  Docker API client, but it's built on `tokio`, which would mean pulling
  an async runtime into a binary whose entire premise ([0013](0013-headless-agent.md))
  is not having one.
- **Skip it, keep `/metrics` as the only endpoint** — the safe default,
  and still the right call for anyone who doesn't need per-container
  comparison; `/containers` is opt-in specifically so this stays
  available to them.
