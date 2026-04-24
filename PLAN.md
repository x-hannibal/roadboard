# Roadboard — Plan

> **Project**: rewrite of Roadboard 2.0 (kept locally in `RB.v2/`, gitignored)
> targeting Rust + Tauri + Axum + SQLite. Branch: `rb-rust`.
> **Scope direction**: EVOLUTION — not a port, not a reset.

## Vision

Operational control plane local-first for project work assisted by AI.
Persists planning (Project / Milestone / Sprint / Task), operational memory
(MemoryEntry / Decision / SessionHandoff), collaboration (multi-user from
day-0), and an architecture graph deterministically derived from LSP via
**Serena as a federated peer MCP**. Two deployment modes share the same UI:
Tauri all-in-one and local-agent + remote-Axum.

Detailed design lives in `docs/`.

## Invariants (do not reopen)

- SQLite as single persistence engine — no Postgres, no external DB
- Zero Docker for development AND end-user deploy
- Zero AI overhead on read-path — all responses deterministic, JSON shape-stable
- MCP as agent transport, single briefing tool only — no `prepare_*` LLM bundles
- CodeFlow built on top of Serena/LSP via federation, not duplicating it
- Multi-user from day-0 in the schema, single-user effective in Tauri mode

## Milestones

### M0 — Repo scaffolding

Set up the Cargo workspace and essential tooling so that subsequent
milestones have a stable foundation to land on.

- [x] Cargo workspace skeleton (`crates/core`, `crates/storage`, `crates/server`, `apps/desktop`, `tools/robot`)
- [x] Root workspace `Cargo.toml` with shared deps and lints
- [x] Next.js scaffold in `apps/web` (routing base, no features)
- [x] `migrations/` directory with placeholder
- [x] `.cargo/config.toml` with shared target dir
- [x] `.gitignore` complete for Rust + Node + Tauri
- [x] `robot` CLI scaffolded (`robot run`, `robot bump`, `--debug`, `--trace`)
- [x] Bash + cmd shims for `robot` at repo root
- [x] CI workflow `.github/workflows/ci.yml`: cargo fmt --check, clippy -D warnings, test, sqlx prepare --check
- [ ] First tag `v0.0.1` via `robot bump 0.0.1`

### M1 — Planning foundation

Schema + core + server + MCP for the planning entity set
(User, Project, ProjectMember, Milestone, Sprint, Task, TaskDependency,
SprintTask). Goal: an agent can run a planning session end-to-end via MCP
without any UI.

#### M1.S1 — Schema + core

- [ ] User entity (migration + core trait + storage impl + tests)
- [ ] MCPToken entity (migration + storage + auth helpers)
- [ ] Project entity (migration + core + storage + tests)
- [ ] ProjectMember entity (migration + core + storage + tests)
- [ ] Milestone entity (migration + core + storage + tests)
- [ ] Sprint entity (migration + core + storage + tests)
- [ ] Task entity (migration + core + storage + tests)
- [ ] TaskDependency entity (migration + storage + tests)
- [ ] SprintTask entity (migration + storage + tests)

#### M1.S2 — Server bootstrap + auth

- [ ] Axum app skeleton with `/health` endpoint
- [ ] Session cookie auth (tower-sessions + SQLite backend)
- [ ] Bearer token auth middleware for `/mcp`
- [ ] CSRF protection for cookie-authenticated mutations
- [ ] REST CRUD endpoints under `/api` for the 8 planning entities
- [ ] Integration test: HTTP create-update-list cycle for tasks

#### M1.S3 — MCP planning tools

- [ ] `initial_instructions` tool (with stub registry)
- [ ] Project tools: `list_projects`, `get_project`, `create_project`
- [ ] Milestone tools: `list_milestones`, `create_milestone`
- [ ] Sprint tools: `list_sprints`, `get_active_sprint`, `create_sprint`, `add_task_to_sprint`, `remove_task_from_sprint`
- [ ] Task tools: `list_tasks`, `get_task`, `create_task`, `update_task`, `update_task_status`
- [ ] Cursor-based pagination implementation for all `list_*` tools
- [ ] Error envelope + stable error codes for all tools
- [ ] Integration test: agent end-to-end flow (create project → milestone → sprint → task → status change) via MCP only

### M2 — Memory & decision layer

Add the memory dimension (MemoryEntry, Decision, SessionHandoff) plus the
unified FTS5 search and the briefing tool.

#### M2.S1 — Schema + core + FTS

- [ ] MemoryEntry entity (migration + core + storage + tests)
- [ ] Decision entity (migration + core + storage + tests)
- [ ] SessionHandoff entity (migration + core + storage + tests)
- [ ] FTS5 `searchable` virtual table + insert/update/delete triggers for memory_entries, decisions, session_handoffs, tasks

#### M2.S2 — REST + MCP tools

- [ ] REST CRUD endpoints for the 3 new entities
- [ ] Memory tools: `search_memory`, `list_memory`, `get_memory_entry`, `create_memory_entry`, `update_memory_entry`
- [ ] Decision tools: `list_decisions`, `get_decision`, `create_decision`, `update_decision`
- [ ] Handoff tool: `create_handoff`
- [ ] Cross-entity search tool: `search`
- [ ] Briefing tool: `get_task_briefing` (without linked nodes — those land in M3)

### M3 — CodeFlow minimal

Materialise just enough of the architecture graph to be useful: stable_id-based
nodes + polymorphic links to operational entities. No code structure storage.

#### M3.S1 — Schema + core

- [ ] ArchitectureNode entity (migration + core + storage + tests)
- [ ] ArchitectureLink entity polymorphic (migration + core + storage + tests)
- [ ] Resolved-status state machine enforcement
- [ ] On-touch `last_seen_at` refresh helper

#### M3.S2 — MCP tools + briefing extension

- [ ] CodeFlow tools: `link_node` (find-or-create), `unlink_node`, `list_node_links`, `list_links_for_entity`, `list_stale_nodes`, `mark_node_status`
- [ ] Update `initial_instructions` to include the verification workflow
- [ ] Update `get_task_briefing` to include linked nodes
- [ ] Integration test: full verification workflow (Topic 6 G) using a Serena mock

### M4 — Web UI

End-to-end browser experience built on the same Axum API. After M4 the
product is usable without any agent.

#### M4.S1 — Auth + navigation shell

- [ ] Login page + cookie session lifecycle
- [ ] Project list page
- [ ] Project detail layout with tabs: Overview / Tasks / Milestones / Sprints / Memory / Decisions / CodeFlow
- [ ] MCP token management page (create, list, revoke)

#### M4.S2 — Planning views

- [ ] Tasks tab: list + filters + create + status flip + detail drawer
- [ ] Milestones tab: list + create + status flip + due date
- [ ] Sprints tab + Sprint board (kanban grouped by task status, for the active sprint)

#### M4.S3 — Memory & decision views

- [ ] Memory tab: list + search + create + archive
- [ ] Decisions tab: list + create + update + supersede chain visualisation

#### M4.S4 — CodeFlow view

- [ ] CodeFlow tab: linked nodes list per project entity
- [ ] Node link form: autocomplete on existing nodes + free-text fallback
- [ ] Stale/broken-node badge + verify-now button (calls back into MCP)

### M5 — Tauri shell

Wrap everything into a Tauri desktop app with embedded server, local-owner
auto-auth, and platform installers.

- [ ] `apps/desktop` Tauri scaffold
- [ ] Spawn embedded `roadboard-server` library on loopback random port
- [ ] Inject `__ROADBOARD_API_BASE__` and `__ROADBOARD_LOCAL_OWNER__` into the WebView
- [ ] Local-owner auto-creation on first launch
- [ ] Bypass login for local-owner (pre-signed session cookie)
- [ ] DB initialisation in platform `app_data_dir`
- [ ] Tauri auto-updater configuration (signing key, `latest.json` endpoint)
- [ ] Installer build for Windows / macOS / Linux
- [ ] First GitHub Release `v0.1.0` (preview)

### Beyond v0.1 — soak toward v1.0

Two-month soak with real usage. Bug fixes, UX polish, telemetry-free
feedback collection. Then cut `v1.0.0` with full code signing on all
platforms and the strict SemVer commitment.

## Deferred (explicit, not in v1)

- Encryption at rest (SQLCipher build feature flag)
- Team entity + TeamMembership (multi-user works without it day-0)
- ActivityEvent / audit log
- Task dependencies advanced UI
- Stdio MCP transport (HTTP only in v1)
- Docker image as distribution channel
- Multi-workspace per single Tauri install
- Real-time collaboration (CRDT, websockets)
- Serena live picker in Tauri UI (the E.3 alternative deferred to Wave 2)
- Release channels (beta vs stable separation)
- Package manager registries (winget, Homebrew, AUR, Flathub)
- Auto-deploy to a hosted instance
- Telemetry / opt-in usage reporting
- In-app crash reporter
