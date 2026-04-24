# Release & distribution — Roadboard 3.0 v1

## Versioning

**SemVer**, strict from `1.0.0` onward.

| Version range | Stability promise |
|---|---|
| `0.y.z` (pre-1.0) | Breaking changes accepted with explicit `CHANGELOG.md` mention |
| `1.0.0+` | Strict SemVer: bump major for breaking, minor for additive, patch for fix |

The `protocol_version` returned by `initial_instructions` follows the same
discipline. Within a `protocol_version`, MCP responses are field-additive.

## Release cadence

- **Pre-1.0**: cut a release whenever a meaningful chunk of work is
  shippable. No fixed cadence.
- **Post-1.0**: cadence to be decided based on real maintenance experience.
  Stable channel only — beta channel deferred until the user base demands it.

## Channels

Single **stable** channel for v1. No beta/preview separation. Pre-1.0 the
`0.y.z` versioning itself signals "preview".

## Artifacts

| Artifact | Built by | Distributed via |
|---|---|---|
| Tauri installer (Win) | `cargo tauri build` on Windows | GitHub Releases |
| Tauri installer (macOS) | `cargo tauri build` on macOS | GitHub Releases |
| Tauri installer (Linux .AppImage / .deb) | `cargo tauri build` on Linux | GitHub Releases |
| Server tarball | `scripts/release-server.sh` (CI) | GitHub Releases |
| `roadboard-server` standalone binary (musl) | `cargo build --release --target x86_64-unknown-linux-musl` | included in tarball |
| Source `.zip` and `.tar.gz` | GitHub Release auto | GitHub Releases |

### Server tarball contents

```
roadboard-server-<version>-linux-x86_64.tar.gz
├── roadboard-server                   musl-static binary
├── static/                            Next.js exported UI
├── migrations/                        SQL files (optional — embedded in binary too)
├── examples/
│   ├── roadboard.service              systemd unit
│   └── Caddyfile                      reverse-proxy + TLS
├── LICENSE
├── README.md
└── CHANGELOG.md
```

### Tauri installer contents

Self-contained: Rust binary + bundled Next.js static files + Tauri WebView
runtime. No external dependencies expected on the user's machine beyond OS
defaults (e.g. WebView2 on Windows is auto-installed by the Tauri installer).

## Code signing

| Phase | macOS | Windows | Linux |
|---|---|---|---|
| Pre-1.0 (`0.y.z`) | unsigned (warning workaround documented) | unsigned (SmartScreen workaround documented) | unsigned tarball/AppImage |
| `1.0.0+` | Apple Developer ID + notarisation | Authenticode (standard or EV) | GPG-signed tarballs |

Pre-1.0 unsigned releases are explicitly marked "preview, manual trust
required" in release notes.

## Update mechanism

### Tauri

- **Built-in updater**: app checks `https://releases.roadboard.dev/latest.json`
  at startup (configurable interval). Downloads update bundle, verifies
  signature, applies on next restart.
- **Manual fallback**: GitHub Releases is always the source of truth.
  Users who disable auto-update download installers manually.
- **Update bundle signing**: Tauri requires update bundles to be signed with
  a separate Tauri signing key (independent of OS code-signing). This is
  free and configured during M5.

### Server

- **No auto-update**. Server admins control deploys.
- Procedure: stop service → replace binary → run `roadboard-server migrate`
  → start service. (Documented in `docs/operations/deploy.md` — Wave 2.)

## Migrations on upgrade

| Environment | Policy |
|---|---|
| Tauri | Auto-run on every startup. Hard-fail with error dialog on failure. User cannot bypass. |
| Server (default) | **Explicit**: `roadboard-server migrate` subcommand. Server refuses to start if pending migrations exist (returns `STORAGE_UNAVAILABLE`). |
| Server with `ROADBOARD_AUTO_MIGRATE=true` | Auto-run on startup. Same hard-fail behaviour. |

### Pre-migration backup (always)

Before applying any migration, both Tauri and server `migrate` copy the DB
file to `<db_path>.pre-<target_version>.bak`. Never overwrites — appends
suffix on collision.

### Rollback procedure

Schema rollback is **never** automatic.

```
1. Stop the process (server or Tauri).
2. Move the corrupt DB file aside:    mv db.sqlite db.sqlite.broken
3. Restore the pre-migration backup:  mv db.sqlite.pre-X.Y.Z.bak db.sqlite
4. Re-install the previous binary version.
5. Start the process.
6. Open a bug report with the corrupt file attached (if shareable).
```

## Bump procedure

Performed by `robot bump` (see `docs/robot.md` for the full spec).
Summary:

1. Working tree must be clean.
2. Determine target version (CLI arg, or auto patch increment).
3. Move `[Unreleased]` content to `## [X.Y.Z] - YYYY-MM-DD` in
   `CHANGELOG.md`. If empty and caller is AI → error.
4. Bump version in workspace `Cargo.toml` and `apps/web/package.json`.
5. Commit, tag `vX.Y.Z`, push with `--follow-tags`.
6. CI workflow (`release.yml`) detects the tag, builds artefacts, attaches
   to a new GitHub Release.

## CHANGELOG discipline

`CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
format with sections: `Added`, `Changed`, `Fixed`, `Removed`, `Deprecated`,
`Security`.

**Updated in the same turn as the code change** — never batched at release
time. The `[Unreleased]` section is the staging area; `robot bump` moves it
to a versioned section.

## Distribution surface (v1)

- **GitHub Releases**: canonical source. All artefacts + checksums + (later)
  signatures.
- **Landing page** with download links: Wave 2.
- **Package managers** (winget, Homebrew, AUR, Flathub): Wave 3+.

## Release CI

Triggered by `git push --tags` matching `v*`:

```
release.yml
├── matrix: [ubuntu-latest, macos-latest, windows-latest]
│   ├── checkout
│   ├── install Rust + Node + Tauri deps
│   ├── cargo tauri build
│   └── upload installer to artefacts
├── linux-server-build
│   ├── checkout
│   ├── cargo build --release --target x86_64-unknown-linux-musl -p roadboard-server
│   ├── pnpm --dir apps/web build && export
│   ├── package tarball with examples + docs
│   └── upload tarball to artefacts
└── publish
    ├── download all artefacts
    ├── compute checksums (sha256)
    ├── create GitHub Release with notes from CHANGELOG.md [X.Y.Z] section
    └── attach all artefacts + checksums
```

CI itself is out of M0 scope (M0 only sets up `cargo fmt` / `clippy` /
`test` / `sqlx prepare --check`). Release workflow lands in M5 alongside the
first Tauri builds.

## What is NOT here in v1

- Docker image (deferred — community can package if there's demand)
- Auto-deploy to a hosted instance (deferred — Wave 3+)
- Telemetry / opt-in usage reporting (deferred — privacy-design conversation
  before any code)
- In-app crash reporter (deferred)
- Code signing for pre-1.0 releases (intentional — see Code signing above)
