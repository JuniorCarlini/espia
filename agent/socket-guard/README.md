# espia-socket-guard

A minimal, read-only Docker socket proxy — the only thing that ever talks
to the real `/var/run/docker.sock`. `espia-headless`'s `GET /containers`
([ADR 0014](../../docs/adr/0014-docker-container-stats.md)) talks to this
instead of the real socket.

It forwards exactly two request shapes:

```
GET /containers/json
GET /containers/<id>/stats
```

Any other path, or any method other than `GET`, gets a `403` — refused
before the code ever opens a connection to the socket. There's no
`ALLOW_*`-style config to get wrong: the allowlist is two `if` checks in
`main.rs`, not a permission system, and there's no write call to the
socket anywhere in this binary's code, reachable or not.

It's a fully standalone Cargo project (its own `[workspace]`, one
dependency — `tiny_http`), not part of the `agent/` workspace, on purpose:
a proxy whose entire job is being trustworthy shouldn't share a dependency
graph or a `Cargo.lock` with anything else, for reasons unrelated to it.

## Running it

Own container, with the *real* socket mounted into **this one only**:

```sh
cd agent/socket-guard
docker build -t espia-socket-guard .
docker run -d -p 2375:2375 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  espia-socket-guard
```

Then point `espia-headless` at it instead of the real socket:

```sh
docker run -d -p 8080:8080 \
  -e ESPIA_TOKEN="$(openssl rand -hex 32)" \
  -e ESPIA_DOCKER_HOST=espia-socket-guard:2375 \
  espia-headless
```

`espia-headless`'s own container never touches `/var/run/docker.sock` at
all in this setup — only `espia-socket-guard`'s does.

| Env var         | Default              | Purpose                        |
| ---------------- | -------------------- | ------------------------------- |
| `BIND`           | `0.0.0.0:2375`        | Address and port to listen on. |
| `DOCKER_SOCKET`  | `/var/run/docker.sock` | The real socket to forward allowlisted requests to. |

## Why not just use a container's `USER` for isolation

It runs as root — it has to, to reach a root-owned socket. That's fine:
the safety property here isn't "this process has low privilege," it's
"there is no code path, at any privilege level, from an incoming request
to a Docker write call." Privilege dropping matters when code *might*
misuse a broad capability it holds; a narrower capability that was never
given at all doesn't need that second line of defense.

## Tested against

- The exact two allowed calls, against a real Docker daemon: both work.
- `POST /containers/create`, `POST /containers/<id>/stop`,
  `POST /containers/<id>/exec`, `GET /images/json`, and a `../` path
  traversal attempt through the container-id segment: all `403`, before
  reaching the socket.
- End to end with `espia-headless` pointed at it via `ESPIA_DOCKER_HOST`.
