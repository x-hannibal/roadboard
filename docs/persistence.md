# Persistence — Roadboard 3.0 v1

## Stack

| concern | choice |
|---|---|
| Engine | SQLite (single-file, embedded) |
| Rust driver | `sqlx` (async, compile-time SQL validation) |
| Journal mode | WAL |
| FTS | FTS5 (built-in) |
| Migration tool | `sqlx migrate` |
| UUID strategy | UUIDv7 stored as `TEXT(36)` |
| Encryption at rest | none (rely on OS-level FDE) |

Rationale and rejected alternatives are summarised in
`CLAUDE.md` § Architectural Decisions.

## File layout

A single SQLite file `roadboard.db` per install/server instance, with all
projects/users discriminated by `project_id`. Identical schema in Tauri and
in standalone server deploys — same migrations, same backup procedure.

| mode | DB path |
|---|---|
| Tauri (Linux) | `~/.local/share/roadboard/db.sqlite` |
| Tauri (macOS) | `~/Library/Application Support/Roadboard/db.sqlite` |
| Tauri (Windows) | `%APPDATA%\Roadboard\db.sqlite` |
| Server | `$ROADBOARD_DB_PATH` (default `./data/roadboard.db`) |

Tauri uses `tauri::api::path::app_data_dir()` to resolve the platform path.

## Connection pool

```rust
SqlitePoolOptions::new()
    .max_connections(8)               // Tauri default; server scales with CPU
    .acquire_timeout(Duration::from_secs(5))
    .connect_with(
        SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5))
            .foreign_keys(true)
            .synchronous(SqliteSynchronous::Normal)
    )
    .await?
```

`PRAGMA foreign_keys=ON` is **mandatory** (off by default in SQLite).
`synchronous=NORMAL` is the WAL-recommended setting (vs `FULL` cost,
no durability loss in practice).

## Migrations

Files live in `/migrations/` at repo root, named `<UTC-timestamp>_<slug>.sql`.

```
migrations/
  20260424120000_init_users_and_tokens.sql
  20260424120100_init_projects.sql
  20260424120200_init_planning.sql
  ...
```

Forward-only. Rollback = author a new forward migration. `sqlx migrate run`
applies pending migrations in order. The migration history is recorded in the
auto-created `_sqlx_migrations` table.

### Auto-migration policy

| environment | policy |
|---|---|
| Tauri | auto-migrate on every startup, error dialog if any fails |
| Server (default) | **explicit** — migrations only run via `roadboard-server migrate` subcommand |
| Server with `ROADBOARD_AUTO_MIGRATE=true` | auto-migrate on startup |

### Pre-migration backup

Before running migrations, the migrate path **always** copies the DB file to
`<db_path>.pre-<target_version>.bak`. Filename includes the version about to
be applied. Never overwrites an existing `.bak` — appends a numeric suffix
if needed (`.pre-0.3.0.bak.1`, etc.).

Recovery procedure: stop process → delete corrupt `db.sqlite` → rename `.bak`
back → re-launch with the previous binary.

## UUID generation

UUIDv7 has 48-bit ms-precision timestamp + 74 bits of randomness. Serializes
to the standard 36-char string format. Crate: `uuid` with `v7` feature.

```rust
use uuid::Uuid;

fn new_id() -> String {
    Uuid::now_v7().to_string()
}
```

Stored as `TEXT(36)` in SQLite for debug-ability via `sqlite3` CLI. The 1.5x
storage overhead vs `BLOB(16)` is irrelevant at our volumes.

## Full-text search

Single virtual table `searchable` aggregates text content from MemoryEntry,
Decision, SessionHandoff, and Task title/description. Maintained via
triggers on each source table.

### Schema

```sql
CREATE VIRTUAL TABLE searchable USING fts5(
    entity_type UNINDEXED,
    entity_id   UNINDEXED,
    project_id  UNINDEXED,
    title,
    body,
    tokenize = 'porter unicode61 remove_diacritics 2'
);
```

### Triggers (one set per source table)

```sql
CREATE TRIGGER memory_entries_after_insert AFTER INSERT ON memory_entries BEGIN
    INSERT INTO searchable(entity_type, entity_id, project_id, title, body)
    VALUES ('memory_entry', new.id, new.project_id, new.title, new.body);
END;

CREATE TRIGGER memory_entries_after_update AFTER UPDATE ON memory_entries BEGIN
    UPDATE searchable
    SET title = new.title, body = new.body
    WHERE entity_type = 'memory_entry' AND entity_id = new.id;
END;

CREATE TRIGGER memory_entries_after_delete AFTER DELETE ON memory_entries BEGIN
    DELETE FROM searchable
    WHERE entity_type = 'memory_entry' AND entity_id = old.id;
END;
```

Repeat for `decisions` (body = `summary || ' ' || rationale`),
`session_handoffs` (body = `summary || ' ' || what_was_done || ' ' || next_steps`),
`tasks` (body = `description`).

### Query pattern

```sql
SELECT entity_type, entity_id, snippet(searchable, 4, '<mark>', '</mark>', '…', 12) AS hit
FROM searchable
WHERE project_id = ?1
  AND searchable MATCH ?2
  AND (?3 IS NULL OR entity_type IN (?3))
ORDER BY rank
LIMIT ?4 OFFSET ?5;
```

## Concurrency

WAL mode handles unlimited concurrent readers + 1 writer. With our write
volume (humans + agents creating tasks/memory), contention is a non-issue.

`busy_timeout=5000ms` covers transient lock contention.

## Backup & portability

- **Manual backup**: copy the DB file. SQLite is safe to copy when the file
  is not being written to. For online backup, use `sqlite3 backup` API.
- **Project export/import**: out of v1 scope. Future: JSON dump of
  `WHERE project_id = ?` rows, importable with FK rewiring.

## Encryption at rest

Not implemented in v1. Rationale: Roadboard data is project metadata (titles,
descriptions, decisions, file paths, operational links) — not credentials or
secrets. Disk-level encryption (FileVault, BitLocker, LUKS) is the
appropriate layer for protecting against device theft.

Future: SQLCipher integration as a build feature flag
(`--features encrypted-storage`).

## Pragmas applied at connection time

```sql
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA foreign_keys=ON;
PRAGMA busy_timeout=5000;
PRAGMA temp_store=MEMORY;
```
