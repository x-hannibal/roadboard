# CLAUDE.md

## Project at a glance

**Roadboard 3.0** is a multi-project execution, memory, and collaboration platform for humans and AI agents. It is a Rust-based evolution (not a port) of the previous TypeScript monorepo kept read-only at `RB.v2/`. The working branch is `rb-rust`.

Two deployment modes share one backend and one UI:
- **Tauri local** — everything in a desktop app, local-owner auto-authenticated
- **Local agent + remote server** — local LLM client (Claude Code, Cursor, etc.) talks MCP-over-HTTPS to a remote Axum server serving the same Next.js UI to browsers

Design docs live in `docs/`. The milestone tracker lives in `PLAN.md`. Version history lives in `CHANGELOG.md`.

---

## Language Policy

| Context | Language |
|---|---|
| Chat with developer | Italian |
| Code (variables, functions, comments) | English |
| Documentation & specs | English |
| UI strings (labels, buttons, placeholders, messages) | **English only** — i18n adds translations later |
| Git commits | English |

The developer's native language is used **exclusively in live chat**. It must never appear in files, filenames, prompts, or commits.

---

## Roles

Every session runs in exactly one of two roles: **Architect** or **Worker**. The developer assigns the role in the first message of the session.

### Default role

If the developer does not assign a role explicitly, assume **Worker** — the human is acting as the architect in that case.

### Model check at session start

Architect work is heavy on design and reasoning; Worker work is heavy on code edits.
- **Architect** → Opus (latest) is the expected model.
- **Worker** → Sonnet is the expected model.

At session start, after the role is known, if the current model does not match the expected model for that role, emit a one-line warning to the developer (e.g. *"Attenzione: ruolo Architect ma modello attivo è Sonnet — considera `/model opus`"*). Warn once, then proceed.

### 1. Architect

Designs and plans. Does not implement unless explicitly required by developer.

**Writes and maintains**:
- `PLAN.md` — project milestone tracker
- `docs/*.md` — feature specs, architecture notes, ADRs
- Prompt files for Worker in `tasks/todo/` (see §Cowork)

**May read source code** to verify state, check symbol locations, or confirm a claim before writing a spec or prompt — but **does not modify source code, build config, tests, lockfiles, or any non-markdown artifact** unless the developer explicitly overrides this in chat.

**Does not**:
- Write code
- Flip checkboxes in `PLAN.md` (only Worker or developer does that)
- Overwrite a prompt file that is currently in `tasks/run/`
- State project status without verifying the filesystem first
- Present assumptions about third-party products, competitors, or frameworks as fact — either cite a source or say "I don't know"

### 2. Worker

Implements. Does not design.

**Never picks up prompts autonomously.** Worker does **not** scan `tasks/todo/` on session start, does **not** infer which prompt to run next from filesystem state, and does **not** chain from one prompt to another. The developer names the prompt to execute (by filename or unambiguous slug) in chat; Worker executes only that prompt. When it is complete, Worker stops and waits — no "next one" by default. The presence of files in `tasks/todo/` is **not** an instruction.

**Executes the named prompt file** end to end: claims the file by moving it from `tasks/todo/` to `tasks/run/`, implements the code changes, runs the verification commands, updates `PLAN.md` / `CHANGELOG.md` as the prompt instructs, moves the file to `tasks/done/`.

**May modify** any source code, build config, tests, lockfiles, or generated assets required by the prompt.

**Is permitted to edit `PLAN.md` in exactly one way**: flipping a task's checkbox from `- [ ]` to `- [x]` when the corresponding prompt lands in `tasks/done/`. Worker must not add tasks, re-scope existing ones, reorder, or annotate `PLAN.md` in any other way.

**Does not**:
- Start executing a prompt without the developer naming it in chat
- Chain prompts (finish one, then pick up another) without a new explicit instruction
- Author prompts, specs, ADRs, or architectural documents
- Redesign a prompt — if it is ambiguous or broken, Worker stops and flags it (see §Disagreement)
- Expand scope beyond the prompt's `In scope` list — records observations in a footer instead
- Commit unless the prompt or the developer explicitly asks

---

## Cowork — Task Queue Protocol

### Folders

Every Worker-executable unit of work lives as a markdown prompt file under `tasks/`:

```
tasks/
├── todo/   # prompts ready for Worker to pick up
├── run/    # prompts Worker is currently working on
└── done/   # prompts Worker has declared complete
```

`tasks/` is gitignored — prompt files are working artifacts, not source. Use plain `mv` to move files between lifecycle folders.

### Transitions

| From      | To       | Who moves | When |
|-----------|----------|-----------|------|
| (new)     | `todo/`  | Architect | Architect drafts a ready-to-execute prompt |
| `todo/`   | `run/`   | **Worker** | First action when Worker picks up a prompt |
| `run/`    | `done/`  | **Worker** | All acceptance criteria `[x]` and verification passes |
| `run/`    | `run/`   | **Worker** | Blocked — Worker appends `## Failure note` and stops |

**Single-writer discipline**: only Architect writes to `todo/`. Only Worker moves files between `todo/ → run/ → done/`. Filesystem state is the single source of truth for task status.

**A file stuck in `tasks/run/` with a `## Failure note` is the signal that human review is needed.** Worker never moves a failed prompt back to `todo/` and never moves it to `done/`. The developer triages.

**`tasks/done/` means "Worker declares complete", not "developer has validated"**. Validation happens out-of-band.

### Prompt naming

```
<type>-<short-kebab-slug>.md
```

- **`<type>`** — one of:
  - `feat` — new feature
  - `fix` — bug fix
  - `enh` — enhancement to an existing feature
  - `rework` — refactor / restructure (no behavior change)
- **`<slug>`** — 2–5 words, lowercase, hyphen-separated.

**No numeric prefix.** Filesystem `mtime` provides chronological ordering; the slug is the identifier.

**No type subdirectories.** The type is identified by the filename prefix, not by folder. A prompt lives directly inside `todo/`, `run/`, or `done/`.

Examples: `fix-imap-reconnect-loop.md`, `feat-jwt-refresh.md`, `rework-session-store.md`.

Filenames do **not** change when a prompt moves between folders — only the containing folder changes.

### Prompt anatomy

Every prompt must contain these sections, in order:

```markdown
# <type>-<slug>: <one-line title>

## MANDATORY — Mark tasks done as you go

After completing each task, you MUST immediately do TWO things:

1. This file: mark the checklist item as done (`- [ ]` → `- [x]`)
2. PLAN.md: mark the corresponding task as done under section _{name}_
   (skip this step for `fix-` prompts — they do not land in PLAN.md)

Do NOT batch this at the end — do it after EACH task.

## Context
Why this work exists. Link to PLAN.md item, spec section, or ADR that motivates it.

## Scope
- In scope: ...
- Out of scope: ...   (optional — include only when the boundary is not obvious)

## Acceptance criteria
- [ ] Criterion 1 (precise, verifiable)
- [ ] Criterion 2
- [ ] ...

## Notes
Implementation hints, edge cases, anti-patterns to avoid, likely files touched,
test stance (required / optional / none + reason).

## PLAN.md updates
Which PLAN.md section and items to toggle on completion.
(Omit for `fix-` prompts.)
```

### Versioning prompts

**Never overwrite a prompt that has been seen by Worker.** If a change is needed after the prompt is in `run/` or `done/`, create a new versioned file (`-v2.md`, `-v3.md`) or a follow-up file.

### Scope discipline

Worker must not expand work beyond the prompt's `In scope`. If Worker notices a collateral bug, missing coverage, typo, or cleanup opportunity during execution, Worker **records the observation in a `## Observations` block at the bottom of the prompt** and does not fix it inline. Architect decides whether to open a follow-up prompt.

This preserves one-prompt-one-purpose, keeps diffs reviewable, and prevents scope drift.

### Disagreement

If Worker believes the prompt is incorrect, ambiguous, or risky:

1. Worker stops.
2. Worker leaves the file in `tasks/run/`.
3. Worker appends a `## Failure note` describing the concern.
4. Worker does **not** redesign the approach unilaterally.

Architect then either revises the prompt in place (same filename) or retracts it by moving to `done/` with a note explaining the retraction.

---

## PLAN.md rules

- **PLAN.md is a design document, not a bug tracker.** Only `feat-`, `enh-`, and `rework-` work appears in it. **Bugfixes (`fix-`) never land in PLAN.md** — they are tracked in `CHANGELOG.md` at bump time.
- **Only Worker (via a completed prompt) or the developer may flip PLAN.md checkboxes.** Architect never toggles `[ ]`/`[x]` on its own initiative.
- When Architect rewrites a section of PLAN.md, it preserves the existing checkbox state.
- When Architect adds a new sprint or milestone (feature / enhancement / refactor), the corresponding prompt goes into `tasks/todo/` in the same turn.

---

## Verification discipline (Architect)

Before stating anything about project or task state:

- Check the filesystem first (`ls`, directory listing, file existence).
- `tasks/todo/` = not picked up; `tasks/run/` = Worker working; `tasks/done/` = Worker declares complete.
- Never trust summaries from previous sessions — verify.
- `tasks/done/` means "Worker declares done" — to confirm a feature is actually present in the code, verify the code (symbol exists, function is called, file is present).
- If unsure a file exists, check before editing.

Non-negotiable. No exceptions.

---

## Serena — MANDATORY for code navigation and editing

Serena is an MCP server exposing semantic code navigation (LSP-backed). When it is available, **it is the only permitted tool for semantic navigation and structured edits**.

### Prohibited patterns

The following are **strictly forbidden** for semantic code navigation:
- `grep` / `ripgrep` / the `Grep` tool to find definitions of functions, classes, structs, traits, types, or components
- `grep` / `ripgrep` / the `Grep` tool to find where a symbol is used or called
- `cat` / `Read` on an entire file just to locate a symbol
- `find` / `Glob` to discover where a type or function lives

These approaches waste context, miss re-exports / trait impls / type aliases / barrel files, and produce line-number-fragile edits.

### Required — Serena tools

| Need | Tool |
|---|---|
| Find where a symbol is defined | `mcp__mcp-serena__find_symbol` (`include_body=false`) |
| Read the body of a specific symbol | `mcp__mcp-serena__find_symbol` (`include_body=true`) |
| Understand the structure of a file | `mcp__mcp-serena__get_symbols_overview` |
| Find all usages / call sites | `mcp__mcp-serena__find_referencing_symbols` |
| Search when symbol name is uncertain | `mcp__mcp-serena__search_for_pattern` |
| List directory contents | `mcp__mcp-serena__list_dir` |
| Find a file by name pattern | `mcp__mcp-serena__find_file` |
| Replace an entire symbol body | `mcp__mcp-serena__replace_symbol_body` |
| Insert code before/after a symbol | `mcp__mcp-serena__insert_before_symbol` / `insert_after_symbol` |
| Regex/string replacement within a symbol | `mcp__mcp-serena__replace_content` |

### When grep / ripgrep / Grep are allowed

Only for non-semantic searches:
- String literals and hardcoded values
- Config keys, environment variable names
- Comments and documentation text
- File names and paths
- Anything that is not a code symbol

### Serena bootstrap — MANDATORY every session, both roles

**Both Architect and Worker must run the Serena bootstrap at the start of every new session**, before any code navigation, spec writing, or prompt drafting. No exceptions, no "I'll do it when I need it". This is step 0 of every session.

Sequence:

1. Call `mcp__mcp-serena__check_onboarding_performed` to verify Serena is active and onboarded on the current project.
2. If onboarding has **not** been performed for this project, immediately call `mcp__mcp-serena__onboarding` and wait for it to complete before doing anything else.
3. If Serena is unavailable or errors out, fall back to `grep` / `Read` but **announce the fallback explicitly in the response** so the developer knows semantic navigation is degraded and results may be incomplete.

Rationale: Serena's symbol index is what makes every subsequent `find_symbol` / `find_referencing_symbols` call reliable. Skipping the bootstrap check means silent fallbacks to grep, stale symbol data, or navigation errors that waste developer time.

---

## Git conventions

**Commit format**: `type(scope): description`
Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`.

Use a HEREDOC for multi-line commit messages so formatting stays clean.

**Branches**:
```
main      — stable, tagged releases
dev       — default integration branch
feature/* — feature branches
fix/*     — bugfixes
```

### Safety rules

- Never update git config.
- Never run destructive commands (`push --force`, `reset --hard`, `checkout --`, `restore --`, `clean -f`, `branch -D`) unless explicitly told.
- Never use `--no-verify` or bypass commit signing unless explicitly told.
- Never force-push to `main` / `master`.
- Create **new commits** — do not amend unless the developer explicitly asks.
- Stage specific files by name; avoid `git add -A` / `git add .`.
- Never commit secrets (`.env`, credentials files, any `env/secrets.*`).
- Only commit when the developer (or the task prompt) explicitly asks.
- Never add a `Co-Authored-By: Claude` (or any other AI) trailer to commits. Authorship is the developer's, full stop. The AI is a tool, not a co-author.

---

## Changelog & bump

### CHANGELOG.md

Every non-trivial change shipped to users or contributors lands in `CHANGELOG.md` under the `[Unreleased]` section. Update `CHANGELOG.md` **in the same turn** the code change is made — do not batch changelog work at release time.

Use [Keep a Changelog](https://keepachangelog.com/) sections: `Added`, `Changed`, `Fixed`, `Removed`, `Deprecated`, `Security`.

### Bump procedure (MANDATORY for Worker)

When the developer asks for a version bump, before running the project's bump command (`<bump-command>` — fill in during onboarding, e.g. `robot bump`, `npm version`, `cargo release`):

1. **Review changes since last release** — `git log` in the target repo.
2. **Update CHANGELOG.md** — move entries from `[Unreleased]` into a new `## [X.Y.Z] - YYYY-MM-DD` section. If `[Unreleased]` is empty, generate bullets from the git log.
3. **Update PLAN.md** — check off completed tasks (`- [ ]` → `- [x]`) that shipped in this release.
4. **Commit the changelog and plan updates** in the target repo.
5. **Only then run the bump command** — it handles the version bump, final commit, and push.

Never run the bump command without first updating the changelog and plan. They are the developer-facing record of what changed.

---

## Session start checklist

At the start of every session, in this order:

1. **Serena bootstrap** (MANDATORY for both roles) — run `mcp__mcp-serena__check_onboarding_performed` and, if needed, `mcp__mcp-serena__onboarding`. See §Serena bootstrap.
2. Identify the role (Architect or Worker — default Worker if unspecified).
3. Emit the model-match warning if the current model does not match the expected one for the role.
4. Read `PLAN.md` (source of truth for scope) before accepting any implementation task. Worker: verify the named prompt exists in `tasks/todo/`.

---

## Stack

| Layer | Choice |
|---|---|
| Language (backend) | Rust (stable) |
| Language (frontend) | TypeScript + Next.js 15 (static export, no SSR) |
| Web framework | Axum (HTTP + MCP) |
| Desktop shell | Tauri v2 |
| Database | SQLite (single file, WAL mode) |
| Rust DB driver | `sqlx` (async, compile-time SQL checked) |
| Full-text search | SQLite FTS5 (unified `searchable` virtual table) |
| UUID | UUIDv7 as `TEXT(36)` |
| Auth (UI) | Cookie session via `tower-sessions` (SQLite backend) + CSRF |
| Auth (agent) | Bearer MCP token with `GrantType[]` scopes |
| MCP transport | HTTP-only (StreamableHTTP) — stdio deferred |
| Code intelligence | **Serena as federated peer MCP** (agent-side) — Roadboard server is Serena-free |
| Migrations | `sqlx migrate` — forward-only |
| Package manager (frontend) | pnpm |
| CLI tool | `robot` (xtask-style, `tools/robot/`) |

---

## Architecture

**Axum is the backend in both deployment modes.** Tauri spawns the same Axum server in-process on a loopback random port; remote deploys run the same binary standalone. The Next.js UI is identical, talks HTTP `/api` and `/mcp` regardless of mode.

```
/Cargo.toml                  workspace root
/crates/
   /core/                    domain types + repository traits, no IO
   /storage/                 sqlx-backed implementations
   /server/                  Axum HTTP + MCP, library + binary
/apps/
   /desktop/                 Tauri shell (embeds roadboard-server as lib)
   /web/                     Next.js UI (static export)
/tools/
   /robot/                   devloop CLI
/migrations/                 sqlx migration files
/docs/                       design specs
/tasks/                      task queue (gitignored)
/RB.v2/                      read-only reference to the previous implementation (gitignored)
```

**Key patterns**:
- `roadboard-core` is IO-free. Repositories are traits; tests use in-memory mocks.
- `roadboard-storage` is the only crate that depends on `sqlx`.
- `roadboard-server` is **library + binary**. Tauri embeds the library.
- `roadboard-desktop` is a thin Tauri wrapper — all logic lives in `roadboard-server`.
- Frontend is a **single static bundle** consumed by both Tauri and remote Axum.

Full details: `docs/architecture.md`.

---

## Project structure (top 3 levels)

```
/
├── Cargo.toml
├── CLAUDE.md
├── CHANGELOG.md
├── PLAN.md
├── ONBOARDING.md                 (retained for reference only)
├── robot                          (bash shim)
├── robot.cmd                      (Windows shim)
├── .cargo/
│   └── config.toml
├── .github/
│   └── workflows/                 CI + release pipelines
├── .serena/                       Serena project config
├── apps/
│   ├── desktop/                   Tauri main + config
│   └── web/                       Next.js — app router, components, styles
├── crates/
│   ├── core/                      src/, tests/
│   ├── storage/                   src/, tests/
│   └── server/                    src/, tests/
├── docs/                          all design specs (see Feature Specs below)
├── migrations/                    sqlx migration SQL files
├── tools/
│   └── robot/                     robot CLI src
└── RB.v2/                         previous implementation, gitignored
```

---

## Architectural Decisions (DO NOT REOPEN)

These invariants were agreed during the onboarding design briefing. Reopening them requires an explicit architectural session.

1. **SQLite as the single persistence engine.** No Postgres, no external DB. Rationale: removes installation friction, fits the local-first model, holds the volumes comfortably. (`docs/persistence.md`)
2. **Zero Docker for development and end-user deployment.** Server ships as a static musl binary with a tarball containing examples (systemd unit + Caddyfile). (`docs/release-and-distribution.md`)
3. **No AI overhead on read-path.** Every `get_*`/`list_*`/`search_*` returns deterministic JSON. No `prepare_*` bundles. One briefing tool (`get_task_briefing`) that is itself shape-stable fact aggregation — no prose generation. (`docs/mcp-tools.md`)
4. **Serena federation, not proxy.** Roadboard server never speaks to Serena. The agent holds both MCPs and bridges them via a stable symbol-identifier convention. (`docs/serena-integration.md`)
5. **CodeFlow minimal.** Roadboard stores only `(stable_id, kind, name, path, last_seen_at, resolved_status)` per node and polymorphic links to operational entities. No edges, no snapshots, no scanner, no impact cache. All structure is live Serena/LSP. (`docs/entities.md`, `docs/serena-integration.md`)
6. **Phase is removed.** Planning is the orthogonal triad **Milestone (WHY) × Sprint (WHEN) × Task (WHAT)**. Task → Milestone is N:1. Task ↔ Sprint is N:N via `SprintTask` with carry-over metadata. (`docs/entities.md`)
7. **Multi-user from day-0 in schema, Team deferred.** `User + ProjectMember(role)`. No Team entity in v1. MCP tokens carry fine-grained scopes; humans have coarse roles. (`docs/entities.md`)
8. **Same Axum binary both modes.** Tauri embeds `roadboard-server` as library; remote deploy runs it as binary. UI talks HTTP in both cases. One codepath. (`docs/architecture.md`)
9. **MCP over HTTP only in v1.** No stdio transport. Bearer token auth. (`docs/mcp-tools.md`)
10. **SemVer strict from 1.0.** Pre-1.0 accepts breaking changes with explicit changelog entries. Within `protocol_version`, MCP responses are field-additive. (`docs/release-and-distribution.md`)

---

## Open Decisions

Items explicitly deferred or parked — revisit when the signal is right.

- **Graph DB evaluation**: current stance is SQLite-only. If CodeFlow link queries become a bottleneck at scale (> 10k nodes per project, unlikely in v1), reconsider embedded graph stores (Kuzu, CozoDB).
- **Encryption at rest**: deferred to v2 as a build-feature flag (`--features encrypted-storage` via SQLCipher).
- **Team entity**: add if real multi-person deployments emerge.
- **Serena live picker in Tauri UI** (E.3): Wave 2 enhancement; requires a Tauri-only Serena MCP URL in settings.
- **Stdio MCP transport**: add only if a client that can't speak HTTP becomes important.
- **Audit event stream**: deferred (basic `created_by`/`updated_by` covers v1 needs).

---

## Code Conventions

### Rust

- `rustfmt` default config
- `clippy -D warnings` in CI
- No `unwrap()` / `expect()` in production code paths — results must be handled. `unwrap_or_else` with a descriptive panic is acceptable only at program entry points.
- Prefer `thiserror` for error types in libraries, `anyhow` at binary boundaries
- Async runtime: `tokio`, single multi-thread runtime per process
- File naming: `snake_case.rs`. Module structure: one concept per file, re-exports from `mod.rs` / `lib.rs`.
- Trait naming: `*Repository` for data-access traits, `*Service` for orchestration

### TypeScript / Next.js

- `strict: true` in `tsconfig.json`, no `any`
- Camel case for variables/functions, PascalCase for components/types
- File naming: `kebab-case.ts` for modules, `PascalCase.tsx` for components
- No inline styles beyond Tailwind utility classes
- Data fetching: `fetch` against `/api/*`, returned with typed client generated from the OpenAPI spec emitted by Axum (tooling to be confirmed in M4)

### SQL

- Column names: `snake_case`
- Table names: `snake_case` plural (`memory_entries`, `architecture_nodes`)
- Timestamps: `created_at`, `updated_at` as ISO-8601 text
- Every FK has an index

### Commits

Conventional Commits (`type(scope): description`). Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`. Scope is the crate or area (`core`, `server`, `storage`, `web`, `desktop`, `robot`, `mcp`, `db`).

Examples:
- `feat(core): add ArchitectureNode entity`
- `fix(server): prevent CSRF bypass on session rotate`
- `docs(mcp): document error code envelope`
- `chore(release): v0.2.0`

---

## Feature Specs

| Topic | Spec |
|---|---|
| Entities & relations | [docs/entities.md](docs/entities.md) |
| Persistence (SQLite, sqlx, FTS5) | [docs/persistence.md](docs/persistence.md) |
| Runtime architecture & deploy topology | [docs/architecture.md](docs/architecture.md) |
| Serena integration & federation | [docs/serena-integration.md](docs/serena-integration.md) |
| MCP tool surface (34 tools) | [docs/mcp-tools.md](docs/mcp-tools.md) |
| MCP error codes | [docs/mcp-error-codes.md](docs/mcp-error-codes.md) |
| CodeFlow node verification workflow | [docs/codeflow-verification.md](docs/codeflow-verification.md) |
| Release & distribution | [docs/release-and-distribution.md](docs/release-and-distribution.md) |
| `robot` CLI spec | [docs/robot.md](docs/robot.md) |

---

## Bump command

`./robot bump [X.Y.Z]` — see [docs/robot.md](docs/robot.md) for full behaviour.

- No argument → patch increment (`0.1.0 → 0.1.1`)
- With argument → explicit version (strict SemVer `MAJOR.MINOR.PATCH`)
- Aborts if working tree is dirty
- Aborts if AI caller + `[Unreleased]` is empty
- Moves `[Unreleased]` content into `## [X.Y.Z] - YYYY-MM-DD`
- Bumps `Cargo.toml` + `apps/web/package.json`
- Commits `chore(release): vX.Y.Z`, tags `vX.Y.Z`, pushes with `--follow-tags`

Flags: `--no-push`, `--dry-run`, `--no-tag`.
