# Design Decisions — Roadboard 3.0 v1

Captured at the end of the onboarding design briefing (Architect session on
2026-04-24). Each item is an invariant for v1 unless explicitly marked
re-openable.

## Stack & topology

- **SQLite** as single persistence engine. No Postgres.
- **sqlx** as the Rust DB driver (async, compile-time SQL checked).
- **Axum** as the backend, same binary in Tauri (embedded) and remote deploy modes.
- **Next.js 15** with static export — no SSR. UI talks HTTP `/api` and `/mcp` ovunque.
- **Tauri v2** as desktop shell, spawns embedded Axum on loopback random port.
- **MCP-over-HTTP only** in v1 (no stdio transport).
- **UUIDv7** as PKs, stored `TEXT(36)`.
- **FTS5** unified `searchable` virtual table for cross-entity search.
- **WAL mode** sempre. `PRAGMA foreign_keys=ON`.
- **Zero Docker** per dev e deploy end-user.

## Architecture

- Workspace crates: `core` (IO-free domain), `storage` (sqlx impl),
  `server` (Axum lib+bin), `tools/robot` (CLI). `apps/desktop` (Tauri) +
  `apps/web` (Next.js). Layout confirmed in `docs/architecture.md`.
- `roadboard-server` is library + binary: Tauri embeds it, deploy runs it.
- **Serena federation** — Roadboard server NEVER talks to Serena. The agent
  holds both MCPs and bridges them via stable symbol identifiers.
- **CodeFlow minimal**: only `(stable_id, kind, name, path, last_seen_at,
  resolved_status)` + polymorphic links stored. No edges, no snapshots, no
  scanner, no impact cache. All structure is live Serena/LSP.
- Stable identifier format: `<posix-relative-path>::<Symbol::Path::Qualified>`.
  Path always POSIX, relative to project root. Roadboard does not validate.

## Data model (14 entities v1)

Planning: `Project`, `Milestone`, `Sprint`, `Task`, `TaskDependency`, `SprintTask`.
Memory: `MemoryEntry`, `Decision`, `SessionHandoff`.
Identity: `User`, `ProjectMember`, `MCPToken`.
CodeFlow: `ArchitectureNode`, `ArchitectureLink`.

Planning hierarchy is **orthogonal**: Milestone (WHY) × Sprint (WHEN) × Task
(WHAT). Task → Milestone is N:1. Task ↔ Sprint is N:N via SprintTask.
**Phase** from RB.v2 is removed.

Multi-user in schema from day-0 (User + ProjectMember with `role` enum).
**Team entity deferred** (add when real team deployments emerge).
MCP tokens carry fine-grained `GrantType[]` scopes; humans have coarse roles.

## MCP interface

34 tools (lean), naming `verb_noun` snake_case, all responses deterministic
JSON with `schema_version`. Cursor-based pagination (opaque base64 cursor,
default 50, max 200). Error envelope `{ error: { code, message, details? } }`
with stable `SNAKE_CASE` codes.

**No `prepare_*` LLM-bundle tools.** One briefing tool only:
`get_task_briefing(taskId)` — fact aggregation, shape-stable, no prose.

Key dropped-from-v2: `prepare_project_summary`, `get_project_changelog`,
`list_phases`, `create_phase`, `update_phase`, `get_architecture_map`,
`get_node_context`. Also dropped for v1 lean: `update_milestone`,
`update_sprint`, `set_task_dependencies`, `list_handoffs`,
`get_latest_handoff`.

`initial_instructions` returns full operational protocol + inline tool
registry (verbose, self-contained).

## Release

- **SemVer strict from 1.0**. Pre-1.0 accepts breaking change with explicit
  CHANGELOG entry.
- **`protocol_version`** in `initial_instructions` — field-additive guarantee
  within major version.
- **Single stable channel**. No beta split in v1.
- **Code signing**: unsigned pre-1.0 (preview); signed post-1.0 across all
  platforms (macOS Developer ID + notarise, Windows Authenticode, Linux GPG).
- **Server deploy**: static musl binary tarball + systemd unit + Caddyfile
  examples. No Docker target in v1.
- **Tauri auto-updater** via `releases.roadboard.dev/latest.json`.
- **Migrations**: auto in Tauri, explicit (`migrate` subcommand) in server
  unless `ROADBOARD_AUTO_MIGRATE=true`.

## Robot CLI

`./robot` (bash + cmd shims) → xtask-style Rust binary at `tools/robot/`.
v1 surface: `run`, `run --server`, `bump [X.Y.Z]`.
Global flags: `-d/--debug`, `-t/--trace`, `-q/--quiet`.
Bump aborts when AI caller + empty `[Unreleased]`. AI detected via
`CLAUDE_CODE=1` or `ROBOT_AI=1`.

## Milestones sequence (M0 → M5)

1. **M0** Repo scaffolding (workspace + robot + CI + v0.0.1)
2. **M1** Planning foundation (8 entities + REST + MCP planning tools)
3. **M2** Memory & decision layer (3 entities + FTS5 + briefing tool)
4. **M3** CodeFlow minimal (2 entities + 6 CodeFlow tools + verify workflow)
5. **M4** Web UI (login + project tabs + linking UI)
6. **M5** Tauri shell (embedded Axum + installers + first v0.1.0 release)

Sprint granularity: **1-2 settimane** per sprint, 2-3 sprint per milestone.

v0.1.0 = completamento M5 (preview). v1.0.0 = dopo ~2 mesi di soak con uso
reale.

## Deferred (do NOT implement without prior design convo)

- SQLCipher encryption at rest
- Team entity
- ActivityEvent / audit log
- Stdio MCP transport
- Docker distribution
- Multi-workspace per install
- Real-time collab (CRDT)
- Serena live picker in Tauri UI
- Release channels (beta/stable split)
- Package manager registries

## References

All details live in `docs/*.md` and `PLAN.md`.
Full reference project: `RB.v2/` (gitignored, read-only sample).
