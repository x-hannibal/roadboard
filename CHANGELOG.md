# Changelog

All notable changes to Roadboard are recorded here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
versioning follows [SemVer](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

