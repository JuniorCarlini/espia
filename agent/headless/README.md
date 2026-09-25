# espia-headless

The same system metrics the desktop agent shows, exposed as a small,
token-gated HTTP endpoint instead of a window — for a VPS or a Docker host
with no display. See [ADR 0013](../../docs/adr/0013-headless-agent.md) for
why this is a separate binary, and why it's deliberately minimal: one
thread, no async runtime, no TLS.

## Running it

```sh
ESPIA_TOKEN=$(openssl rand -hex 32) cargo run -p espia-headless
```

or with Docker:

Run from `agent/` — the Docker context has to include the sibling `core/`
crate, not just `headless/`:

```sh
cd agent
docker build -t espia-headless .
docker run -p 8080:8080 -e ESPIA_TOKEN="$(openssl rand -hex 32)" espia-headless
```

| Env var               | Default              | Purpose                                            |
| --------------------- | -------------------- | --------------------------------------------------- |
| `ESPIA_TOKEN`         | *(none)*             | Required to read `/metrics` and `/containers`. Unset means every request to them is rejected — see below. |
| `ESPIA_BIND`          | `0.0.0.0:8080`       | Address and port to listen on.                     |
| `ESPIA_DOCKER_SOCKET` | `/var/run/docker.sock` | Docker socket path, for `/containers`.           |

## Endpoints

- `GET /health` — always `200 ok`, no token required. Point a Docker or
  orchestrator health check at this.
- `GET /metrics` — the current system snapshot as JSON (CPU, memory,
  disks, network, and top processes), requires
  `Authorization: Bearer <ESPIA_TOKEN>`. Without a valid token: `401`.
  Inside a container, `top_processes` only ever shows this agent's own
  processes — Docker's default PID namespace hides everything else. CPU,
  memory, disk, and network totals aren't affected; see
  [ADR 0014](../../docs/adr/0014-docker-container-stats.md).
- `GET /containers` — every running container's CPU % and memory usage,
  sorted highest CPU first, same token requirement as `/metrics`. Needs
  `/var/run/docker.sock` mounted into the container — without it, this
  returns `502` with an explanation rather than the container list.
  **Read the Security model section below before mounting that socket.**

```sh
curl -H "Authorization: Bearer $ESPIA_TOKEN" http://localhost:8080/metrics
curl -H "Authorization: Bearer $ESPIA_TOKEN" http://localhost:8080/containers
```

To enable `/containers`, mount the socket when running the container:

```sh
docker run -p 8080:8080 \
  -e ESPIA_TOKEN="$(openssl rand -hex 32)" \
  -v /var/run/docker.sock:/var/run/docker.sock \
  espia-headless
```

## Security model

The token is the *only* access control — there's no TLS, no login, no
per-client accounts, on purpose (see the ADR). A few consequences worth
being deliberate about:

- **If `ESPIA_TOKEN` isn't set, the server still starts** (so a
  misconfigured deployment shows up in its own logs instead of refusing to
  boot), but it fails closed: every request to `/metrics` is rejected
  until a token is set.
- **It binds `0.0.0.0` by default.** That's required for Docker's `-p`
  port mapping to reach it at all — a container's network namespace makes
  "just bind localhost" meaningless. It also means this token is your only
  line of defense on an untrusted network; put a firewall or a reverse
  proxy (for TLS) in front of it if the host is reachable from anywhere
  you don't control.
- The token is sent as a plain bearer token — fine over a private network
  or behind a TLS-terminating proxy, not fine sent bare over the public
  internet.

**`/containers` is a bigger trust boundary than the rest of this binary.**
Mounting `/var/run/docker.sock` in gives this container root-equivalent
control of the entire Docker host, not just read access to stats — see
[ADR 0014](../../docs/adr/0014-docker-container-stats.md). Only mount it
on a VPS you already fully control, never on shared or multi-tenant Docker
infrastructure. Leaving the socket unmounted keeps the agent at
[ADR 0013](../../docs/adr/0013-headless-agent.md)'s original, much smaller
blast radius — `/metrics` and `/health` don't need it.
