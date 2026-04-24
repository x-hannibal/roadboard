# `robot` — devloop CLI

`robot` is the project's local devloop tool. It wraps common operations
(running the app, bumping versions, managing the changelog) so that no one
has to remember a long list of `cargo` / `git` / `pnpm` invocations.

## Installation & invocation

`robot` is implemented as an xtask-style Rust binary in
`tools/robot/`. Two shim files at the repo root invoke it:

| Platform | Shim | Implementation |
|---|---|---|
| Linux / macOS | `./robot` (bash) | `exec cargo run -q -p robot --release -- "$@"` |
| Windows | `.\robot.cmd` (cmd) | `@cargo run -q -p robot --release -- %*` |

No global install needed. The binary builds on first run and is then cached
by Cargo.

## Surface (v1)

```
robot run                         # Tauri dev (default)
robot run --server                # Axum standalone dev
robot bump [VERSION]              # version bump + changelog + git tag + push
```

## Global flags

| Flag | Effect |
|---|---|
| `-d`, `--debug` | sets `RUST_LOG=debug` for the spawned subprocess |
| `-t`, `--trace` | sets `RUST_LOG=trace` (wins over `--debug` if both given) |
| `-q`, `--quiet` | sets `RUST_LOG=warn` |
| `--help` | usage |
| `--version` | print robot version |

Default log level when none given: `RUST_LOG=info`.

## `robot run`

| Invocation | Effect |
|---|---|
| `robot run` | `cargo tauri dev` — Tauri shell with Next.js dev server (Tauri's `beforeDevCommand` handles the UI side) |
| `robot run --server` | `cargo watch -q -x "run -p roadboard-server -- serve"` — Axum on default port with hot reload |

Optional flags for `--server` mode:

| Flag | Effect |
|---|---|
| `--with-web` | also spawn `pnpm --dir apps/web dev` concurrently (proxied) |
| `--port <N>` | override the server bind port |

(The `--with-web` integration detail is finalised when M1.S2 lands.)

## `robot bump [VERSION]`

The single source of truth for cutting a release.

### Behaviour

1. **Validate working tree clean** — `git status --porcelain` must be empty.
   Abort with exit 1 otherwise.
2. **Determine target version**:
   - If a positional arg is provided, parse it as SemVer. Reject if not strict
     SemVer (e.g. `0.2`, `1.0.0-beta`) — only `MAJOR.MINOR.PATCH` accepted in v1.
   - Otherwise, read the current version from workspace `Cargo.toml` and
     increment the patch (`0.1.0 → 0.1.1`).
3. **Read `CHANGELOG.md` `[Unreleased]` section**:
   - If empty AND caller is AI (`CLAUDE_CODE=1` or `ROBOT_AI=1` env var):
     **abort** with `EMPTY_UNRELEASED` error and exit 1. The AI is expected
     to update the changelog as part of its workflow per `CLAUDE.md`.
   - If empty AND caller is human: print warning, ask interactively (TTY)
     or proceed (non-TTY).
4. **Transform `CHANGELOG.md`**:
   ```
   ## [Unreleased]                     ## [Unreleased]
   ### Added                           
   - Foo                                ## [X.Y.Z] - YYYY-MM-DD
   ### Fixed                           ### Added
   - Bar                          →    - Foo
                                       ### Fixed
                                       - Bar
   ```
   The new `[Unreleased]` section is left empty.
5. **Bump version**:
   - workspace `Cargo.toml` (`[workspace.package] version = "X.Y.Z"`)
   - `apps/web/package.json` if present
   - any crate that does not use `version.workspace = true` (warned if
     found — should be exceptional)
6. **Commit**:
   ```
   git add CHANGELOG.md Cargo.toml apps/web/package.json
   git commit -m "chore(release): vX.Y.Z"
   ```
7. **Tag**:
   ```
   git tag vX.Y.Z -m "vX.Y.Z"
   ```
8. **Push** (unless `--no-push`):
   ```
   git push origin <current-branch> --follow-tags
   ```

### Flags

| Flag | Effect |
|---|---|
| `--no-push` | skip the final push (useful for review) |
| `--dry-run` | print what would happen, perform no side effects |
| `--no-tag` | skip git tag (rare; explicit) |

### AI detection

The AI-context check at step 3 reads, in order:

1. `ROBOT_AI=1` (explicit override, set by anything that wants to declare AI context)
2. `CLAUDE_CODE=1` (set by Claude Code automatically)
3. `CURSOR_AGENT=1` (set by Cursor agents — placeholder, may need verification)

If any is `1` or `true`, the strict empty-changelog check applies.

## Out of scope (v1)

The following are **not** implemented in `robot` v1. They become relevant
in later waves and will be added incrementally:

| Future command | Wave | Purpose |
|---|---|---|
| `robot deploy` | 3+ | Push the server binary to a configured remote |
| `robot promote` | 3+ | Move artefacts between channels |
| `robot release` | 2 | Trigger CI release pipeline manually |
| `robot db migrate` | 2 | Wrapper around `sqlx migrate run` |
| `robot db studio` | 2 | Open a DB browser |
| `robot init` | — | Interactive project scaffolding (no, ship a template instead) |

When a need is real, we add. No speculative commands.

## Implementation notes

- Crate `tools/robot` uses `clap` (derive API) for argument parsing,
  `git2` for git operations (avoids shelling out), `toml_edit` for
  preserving Cargo.toml formatting on version bumps, `serde_json` for
  package.json, and `regex` (or a tiny hand-rolled parser) for
  CHANGELOG.md transformation.
- All shell-out (when unavoidable, e.g. `cargo tauri dev`) goes through
  `std::process::Command` with proper `RUST_LOG` env injection.
- Cross-platform path handling via `std::path::PathBuf` — never string
  concatenation.
- Exit codes: `0` success, `1` user error / aborted, `2` internal error.

## Examples

```bash
# Start Tauri dev with debug logs
./robot run -d

# Start Axum server with trace logs and Next.js dev concurrently
./robot run --server --with-web -t

# Patch bump (0.1.0 -> 0.1.1)
./robot bump

# Explicit version, dry run
./robot bump 0.2.0 --dry-run

# Explicit version, do not push (review locally first)
./robot bump 0.2.0 --no-push
```
