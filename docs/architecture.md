# Architecture — Roadboard 3.0 v1

## North star

**Axum is the backend in both deployment modes.** Tauri does not ship its
own custom IPC API; it spawns the same Axum HTTP server in-process on a
loopback random port. The Next.js UI is identical in both modes and talks
HTTP `/api` and `/mcp` regardless of where it runs.

One backend surface. One MCP transport. One UI codebase. Zero
"if-tauri-then-X" branches in the frontend.

## Workspace layout

```
/Cargo.toml                  workspace root
/crates/
   /core/                    roadboard-core    — domain types, repository traits, business logic
   /storage/                 roadboard-storage — sqlx implementation of Repository traits
   /server/                  roadboard-server  — Axum HTTP + MCP-over-HTTP, library + binary
/apps/
   /desktop/                 roadboard-desktop — Tauri shell, depends on roadboard-server as library
   /web/                     Next.js UI (not a Cargo crate)
/tools/
   /robot/                   robot CLI (xtask-style)
/migrations/                 sqlx migration files
/.cargo/config.toml          shared target dir + workspace defaults
/robot                       bash shim → cargo run -q -p robot --
/robot.cmd                   Windows shim
```

### Crate responsibilities

| crate | depends on | exports |
|---|---|---|
| `roadboard-core` | nothing project-internal | domain types (`Project`, `Task`, `Sprint`, …), repository traits (`ProjectRepository`, `TaskRepository`, …), business operations |
| `roadboard-storage` | `roadboard-core`, `sqlx` | sqlx-backed implementations of the traits, migrations bootstrap |
| `roadboard-server` | `roadboard-core`, `roadboard-storage`, `axum`, `tower-sessions` | library: HTTP handlers, MCP handlers, app builder; binary: `roadboard-server` with `migrate`, `serve`, `version` subcommands |
| `roadboard-desktop` | `roadboard-server` | Tauri main, spawns embedded server, manages window |
| `tools/robot` | `clap`, `git2`, `toml_edit` | the `robot` CLI |

### Why this split

- `roadboard-core` knows nothing about persistence or transport. Can be
  unit-tested with mocks, can be reused by alternate transports if ever needed.
- `roadboard-storage` is the only crate that touches sqlx. Swappable in tests
  with an in-memory implementation.
- `roadboard-server` is **both library and binary**. Tauri embeds it as a
  library; standalone deploys use the binary.
- `roadboard-desktop` is a thin glue (~few hundred lines). All the smarts
  live in `roadboard-server`.

## Runtime topology

### Mode 1 — Tauri local

```
┌────────────────────────────────────────────────────┐
│ Tauri process                                      │
│ ┌────────────────────────┐ ┌─────────────────────┐ │
│ │ Tauri main             │ │ Embedded Axum       │ │
│ │ (roadboard-desktop)    │ │ (roadboard-server)  │ │
│ │ - spawns Axum          │ │ on 127.0.0.1:<port> │ │
│ │ - injects api base URL │ │ - REST /api         │ │
│ │ - manages window       │ │ - MCP  /mcp         │ │
│ └─────────┬──────────────┘ └────────┬────────────┘ │
│           │                         │              │
│ ┌─────────▼─────────────────────────▼────────────┐ │
│ │ WebView (Tauri)                                │ │
│ │   Next.js UI (static export, bundled)          │ │
│ │   talks HTTP to 127.0.0.1:<port>/api           │ │
│ └────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────┘

Local agent (Claude Code, etc.) configured with MCP at:
   url: http://127.0.0.1:<port>/mcp
   bearer: <user-issued-token>

Local Serena MCP (user-managed, peer to Roadboard) — see
docs/serena-integration.md.
```

### Mode 2 — Local agent + remote server

```
┌────────────────────────────────────────────────────┐ ┌─────────────────────┐
│ Server (deploy box)                                │ │ Developer machine   │
│ ┌────────────────────────────────────────────────┐ │ │                     │
│ │ roadboard-server (standalone binary)           │ │ │ ┌─────────────────┐ │
│ │ - REST /api  (cookie session for browser UI)   │◄┼─┼─┤ Browser         │ │
│ │ - MCP  /mcp  (bearer token for agents)         │◄┼─┼─┤ (Next.js UI)    │ │
│ │ - serves static Next.js UI                     │ │ │ └─────────────────┘ │
│ │ - SQLite at $ROADBOARD_DB_PATH                 │ │ │                     │
│ └────────────────────────────────────────────────┘ │ │ ┌─────────────────┐ │
│           ▲                                        │ │ │ Agent (Claude)  │ │
│           │                                        │◄┼─┤ MCP → server/mcp│ │
│           │ HTTPS via Caddy/nginx                  │ │ │ MCP → local     │ │
└───────────┼────────────────────────────────────────┘ │ │       Serena    │ │
                                                       │ └─────────────────┘ │
                                                       └─────────────────────┘
```

The remote server has zero knowledge of Serena. Federation lives entirely on
the developer machine — the agent talks to both MCPs and uses Roadboard's
`stable_id` convention to bridge.

## Frontend (Next.js)

- **Build**: `next build && next export` produces a static HTML/JS bundle.
- **Routing**: client-side, no SSR, no Node runtime in deploy.
- **API base URL discovery**:
  - Tauri: injected at window load via `window.__ROADBOARD_API_BASE__` set by
    a Tauri command result before the WebView loads pages.
  - Server: same-origin, fixed at `/api`.
- **Auth UI behaviour**:
  - Tauri: `window.__ROADBOARD_LOCAL_OWNER__ === true` → skip login page,
    auto-redirect to project list.
  - Server: standard login page with cookie session.

## Auth model

| principal | mechanism | issued by |
|---|---|---|
| Browser session (UI user) | cookie HttpOnly + SameSite=Lax + CSRF token for mutations | login form (POST `/api/auth/login`) |
| MCP token (agent) | `Authorization: Bearer <token>` | UI "Tokens" page; shown once at creation, hashed at rest |
| Tauri local owner | shared secret env var passed to embedded server at spawn → server returns a pre-signed session cookie loaded into the WebView | Tauri main process at startup |

**Localhost is not trusted by default.** Even in Tauri mode, the embedded
Axum binds `127.0.0.1` and requires a bearer token for `/mcp`. Other local
processes cannot impersonate the agent without the token.

## MCP transport

HTTP only — StreamableHTTP. `POST /mcp` for request/response, `GET /mcp` for
SSE stream. Same protocol surface in Tauri and remote modes. No stdio
transport in v1 (deferred).

## Build matrix

| artifact | command | output |
|---|---|---|
| Server binary | `cargo build --release -p roadboard-server` | `target/release/roadboard-server` (musl static on Linux for portability) |
| Web bundle | `pnpm --dir apps/web build && pnpm --dir apps/web export` | `apps/web/out/` |
| Tauri app | `cargo tauri build` | platform-specific installer in `target/release/bundle/` |
| robot | `cargo build --release -p robot` | `target/release/robot` |

## Deploy guidance

**Server**: ship a tarball containing the binary, the `static/` UI export,
the `migrations/` directory, an example `roadboard.service` systemd unit,
and an example `Caddyfile`. No Docker artifact in v1.

**Tauri**: signed installers per platform via GitHub Releases, Tauri
auto-updater configured against a static `latest.json` at a known URL.

See `docs/release-and-distribution.md` for full details.
