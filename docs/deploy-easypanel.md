# Deploying to Easypanel

Step-by-step for running `espia-headless` (and, optionally,
`espia-socket-guard` for per-container stats) on a VPS through
[Easypanel](https://easypanel.io/). Written from an actual deployment,
end to end — see [ADR 0013](adr/0013-headless-agent.md) and
[ADR 0014](adr/0014-docker-container-stats.md) for why these two
services exist and what trade-off each one makes.

## Prerequisites

- The repo pushed to GitHub (Easypanel builds from a Git source).
- Easypanel's GitHub integration authorized for the repo.

## 1. Deploy `espia`

1. **`+ Serviço` → Aplicativo**
2. **Fonte** tab → **Github** → pick the repo, branch `main`
3. **Caminho de Build**: `/agent` — must start with `/`, or Easypanel
   rejects it as invalid.
4. Save; a **Construção** section appears. Method: **Dockerfile**.
   **Arquivo**: `Dockerfile` (it lives at `agent/Dockerfile`, so this is
   just the filename, not a path).
5. **Ambiente** tab: add
   ```
   ESPIA_TOKEN=<a long random value, e.g. `openssl rand -hex 32`>
   ```
6. **Domínios** tab: Easypanel assigns a domain automatically, already
   pointed at container port `8080` — no change needed there.
7. **Implantar**.

Verify:

```sh
curl https://<your-domain>/health
curl -H "Authorization: Bearer $ESPIA_TOKEN" https://<your-domain>/metrics
```

`/metrics` works from here on its own — the rest of this guide is only
needed for the optional `GET /containers` endpoint (per-container
CPU/memory, not just the VPS totals `/metrics` already gives you).

## 2. (Optional) Deploy `espia-socket-guard`, for `GET /containers`

`/containers` needs a connection to the Docker daemon. Giving `espia`
that directly means giving the container exposed on the public internet
root-equivalent control of the whole Docker host — see ADR 0014 for why
that's the wrong trade-off. `espia-socket-guard` is a second, small
service that holds that access instead, and only forwards the two
read-only calls `/containers` actually needs.

1. **`+ Serviço` → Aplicativo**, source **Github**, same repo and branch.
2. **Caminho de Build**: `/agent/socket-guard`
3. **Construção**: Dockerfile, **Arquivo**: `Dockerfile`.
4. **Armazenamento** tab → **Adicionar Bind Mount**:
   - Host path: `/var/run/docker.sock`
   - Mount path: `/var/run/docker.sock`
5. No domain needed — only `espia` reaches this, over Easypanel's
   internal network by service name. Skip the Domínios tab.
6. **Implantar**. Note the exact service name Easypanel gives it (shown
   at the top of the page, e.g. `<project>_espia-socket-guard`) — the
   next step needs it.

## 3. Point `espia` at the proxy

Back on the `espia` service:

1. **Ambiente** tab: add
   ```
   ESPIA_DOCKER_HOST=<espia-socket-guard's service name>:2375
   ```
2. **Armazenamento** tab: if you mounted `/var/run/docker.sock` into
   `espia` directly while testing, remove that bind mount now — `espia`
   no longer needs it once it's talking to the proxy instead.
3. **Implantar**.

Verify:

```sh
curl -H "Authorization: Bearer $ESPIA_TOKEN" https://<your-domain>/containers
```

Should return every running container on the VPS, sorted by CPU,
`espia` and `espia-socket-guard` included.

## Troubleshooting

- **"Caminho de Build" rejected as invalid** — it needs a leading `/`
  (e.g. `/agent`, not `agent`).
- **`/containers` returns `502`** — `espia` can't reach the socket or the
  proxy. Check `ESPIA_DOCKER_HOST` matches the proxy service's exact
  name, and that the bind mount on `espia-socket-guard` is actually
  saved (re-open Armazenamento and confirm it's still listed after a
  redeploy).
- **`/containers` is slow** — make sure both services are running a
  build from `dda875c` or later; earlier builds fetched every
  container's stats one at a time, which scales linearly with container
  count (measured ~24s for 12 containers, fixed to ~2s).

## Updating

Both services redeploy from whatever's on `main`. After pushing a
change, redeploy each affected service from its **Implantações** tab
(or wherever Easypanel exposes it) — there's no auto-deploy configured
by default.
