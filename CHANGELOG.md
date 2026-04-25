# Changelog

All notable changes to Roadboard are recorded here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
versioning follows [SemVer](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `roadboard-core`: domain types and repository traits for User, McpToken, Project, ProjectMember, Milestone, Sprint, Task, TaskDependency, SprintTask
- `roadboard-storage`: sqlx-backed SQLite implementations for all 9 planning entities; Argon2 password hashing; SHA-256 MCP token hashing
- SQLite migrations: users + tokens, projects + members, planning entities (milestones, sprints, tasks, dependencies, sprint_tasks)
- `roadboard-server`: Axum app with cookie session auth (tower-sessions + SQLite store), double-submit CSRF middleware, bearer token auth middleware
- REST CRUD endpoints under `/api` for all 8 planning entities + auth + token management
- MCP transport at `POST /mcp` (JSON-RPC 2.0 dispatcher) and `GET /mcp` (SSE stub) behind bearer auth
- 16 MCP planning tools: `initial_instructions`, `list/get/create_project`, `list/create_milestone`, `list/get_active/create_sprint`, `add/remove_task_from_sprint`, `list/get/create/update/update_status_task`
- Cursor-based pagination (base64url JSON) on all `list_*` tools and REST endpoints
- Stable MCP error envelope: FORBIDDEN, PROJECT_ACCESS_DENIED, NOT_FOUND, VALIDATION_ERROR, INVALID_ENUM, INVALID_REFERENCE, INVALID_CURSOR, CONFLICT, TASK_ALREADY_IN_SPRINT, TASK_PROJECT_MISMATCH, INTERNAL_ERROR
- Integration tests: HTTP auth cycle, CRUD tasks, CSRF protection, MCP end-to-end planning flow (44 tests, 0 failures)

## [0.0.1] - 2026-04-24

### Added
- Cargo workspace with `crates/core`, `crates/storage`, `crates/server`, `tools/robot` and shared `[workspace.dependencies]`
- Root `Cargo.toml` with workspace-level version `0.0.1`, edition 2021, rust-version 1.82, and clippy/rust lints
- `.cargo/config.toml` with shared `target-dir`
- `roadboard-core` crate — IO-free domain types scaffold (empty lib with module doc)
- `roadboard-storage` crate — sqlx-backed repository implementations scaffold
- `roadboard-server` crate — Axum HTTP server with `GET /health → { "status": "ok" }` and `serve`/`migrate`/`version` subcommands
- `robot` CLI (`tools/robot`) with `run`, `run --server`, `bump [VERSION]`, and `-d`/`-t`/`-q` global flags; full bump logic including CHANGELOG.md transformation, SemVer validation, git commit/tag/push, and AI-caller empty-changelog guard
- Bash shim `robot` and Windows shim `robot.cmd` at repo root
- `apps/web` Next.js 15 + React 19 + Tailwind 4 scaffold with `output: "export"` static build
- `migrations/` directory with `.gitkeep` placeholder
- `.gitignore` extended for Rust (`/target/`), Node (`node_modules`, `.next`, `out`), and Tauri
- `.github/workflows/ci.yml` with `fmt`, `clippy`, `test`, and `web-build` jobs

