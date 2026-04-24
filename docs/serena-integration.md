# Serena integration — Roadboard 3.0 v1

## Position

Serena and Roadboard are **peer MCP servers**. The agent (Claude Code,
Cursor, Codex, etc.) connects to both and coordinates queries across them
using a shared symbol-identifier convention.

**Roadboard server has zero runtime dependency on Serena.** The federation
lives entirely on the agent side.

```
Developer machine                      
                                       
  ┌─────────────────────────────┐      
  │ Agent (LLM client)          │      
  │  MCP config:                │      
  │   - serena (local stdio)    │      
  │   - roadboard (HTTP, local  │      
  │     or remote)              │      
  └────┬───────────────┬────────┘      
       │               │               
       ▼               ▼               
  ┌────────┐      ┌──────────────────┐ 
  │ Serena │      │ Roadboard server │ 
  └────────┘      └──────────────────┘ 
                                       
  Local code      SQLite (operational  
  filesystem      state only — links,  
                  stable_ids, no LSP   
                  data)                
```

## Why this shape

Considered alternatives:

- **Roadboard-as-Serena-proxy** (Roadboard server speaks MCP to Serena):
  rejected because in remote-Axum mode the *server* has no access to the
  developer's local code. Tunnelling Serena calls back to the client is a
  networking nightmare and a separate Serena lifecycle on the server is
  pointless duplication.
- **Roadboard embeds LSP directly** (rust-analyzer, vtsls, taplo, marksman
  invoked from the Rust core): rejected because it duplicates Serena's
  existing aggregation work and adds ~tens of thousands of lines to maintain.

Federation is the only design that works identically in both deploy modes.

## Stable identifier convention

`ArchitectureNode.stable_id` is the bridge. It must be a string that:
1. Roadboard treats as opaque (no parsing).
2. Serena's `find_symbol` tool can resolve.

Format:

```
<posix-relative-path>::<Symbol::Path::Qualified>
```

### Rules

| rule | example |
|---|---|
| Path is **always POSIX** (forward slashes), even on Windows | `crates/core/src/auth.rs` not `crates\core\src\auth.rs` |
| Path is **relative to project root** | `crates/core/src/auth.rs` not `/home/x/project/crates/core/src/auth.rs` |
| No leading `./` | `Cargo.toml` not `./Cargo.toml` |
| Symbol qualifier separator is `::` | `auth::AuthService::verify_token` |
| For non-symbolic files (Markdown, TOML, plain text), the path alone is the stable_id | `docs/architecture.md` |
| Case-sensitive | tracks exactly what Serena returns |
| No line/column fragment | the symbol is the unit, not the line |

### Examples

| Symbol | stable_id |
|---|---|
| Rust method `verify_token` on `AuthService` in `crates/core/src/auth.rs` | `crates/core/src/auth.rs::AuthService::verify_token` |
| TypeScript React component `Sidebar` in `apps/web/components/Sidebar.tsx` | `apps/web/components/Sidebar.tsx::Sidebar` |
| Whole `Cargo.toml` file | `Cargo.toml` |
| Whole markdown doc | `docs/architecture.md` |
| Rust trait `ProjectRepository` in `crates/core/src/repo/project.rs` | `crates/core/src/repo/project.rs::ProjectRepository` |

## Roadboard side

Roadboard stores only:

- `ArchitectureNode(stable_id, kind, name, path, last_seen_at, resolved_status)`
- `ArchitectureLink(node ↔ {Task | Milestone | Decision | MemoryEntry})`

Roadboard **never** stores:

- Code structure (edges, imports, dependencies, references)
- Symbol bodies, hover content, type information
- Snapshots / versions of the graph
- Impact analyses

Anything code-derived is fetched live from Serena by the agent.

## Resolved-status state machine

```
unverified  ──[mark_node_status: resolved]──→  resolved
unverified  ──[mark_node_status: broken]────→  broken

resolved    ──[TTL exceeded]────────────────→  stale
resolved    ──[mark_node_status: broken]────→  broken

stale       ──[mark_node_status: resolved]──→  resolved
stale       ──[mark_node_status: broken]────→  broken

broken      ──[mark_node_status: resolved]──→  resolved   (rare — re-appeared after revert/rebase)

(any)       ──[hard delete]─────────────────→  row removed (CASCADE on links)
```

Default state at find-or-create: `unverified`.

## Node lifecycle

### Find-or-create on link

When `link_node(projectId, stable_id, entity_type, entity_id, link_type)` is
called and the node does not exist for `(projectId, stable_id)`:

1. Roadboard creates the node with:
   - `stable_id` = as provided
   - `path` = everything before the first `::` in stable_id, or full stable_id if no `::` present
   - `name` = last `::`-segment, or the basename of `path` if no `::` present
   - `kind` = `unknown`
   - `resolved_status` = `unverified`
   - `last_seen_at` = `now()`
2. Then creates the link.

The `kind` is left as `unknown` until an agent verifies the symbol via Serena
and posts back via `mark_node_status` with the kind hint (future enhancement —
v1 leaves `kind` as the agent supplies it).

### TTL-based stale detection

Configurable cutoff (default 7 days). The `list_stale_nodes` MCP tool
returns nodes with:

- `last_seen_at < now() - max_age_days`, OR
- `resolved_status = unverified`

regardless of age (unverified nodes are always stale candidates).

### On-touch refresh

Whenever a node is "touched" by any operation (link created, link queried,
included in a briefing), Roadboard updates `last_seen_at = now()`. The
status itself is not changed by touch — only the timestamp.

## Verification workflow (agent-side)

This is the canonical workflow the agent should run at session start (or on
demand). It is documented machine-readably in
`docs/codeflow-verification.md` and surfaced through `initial_instructions`.

```
1. agent → roadboard.list_stale_nodes(projectId, maxAgeDays=7)
       ← [{ stable_id, kind, name, path, last_seen_at, resolved_status }, ...]

2. for each node:
     # split stable_id at first '::' into (path, qualifier)
     # qualifier may be empty for non-symbol files
     agent → serena.find_symbol(name_path=qualifier, relative_path=path,
                                 include_body=false)
            ← either resolves to a symbol or returns empty

3. agent → roadboard.mark_node_status(projectId, stable_id,
                                       "resolved" | "broken")
       ← updated node

(optional)
4. agent → roadboard.unlink_node(linkId)
       for any "broken" nodes whose links the user/agent deems irrecoverable.
```

## UI picker (v1)

The web UI's "link a node to a Task/Decision/etc." flow uses **autocomplete
on already-known nodes** (querying the local `architecture_nodes` table by
`name`/`path`) plus a **free-text input** for new stable_ids.

A live Serena-backed symbol picker (the UI calling Serena to search) is
**deferred to Wave 2**. It would only work in Tauri mode (Serena lives on the
same machine) and would require a UI-side Serena MCP URL configuration —
worth designing carefully later, not crash-bolting onto v1.

## Failure modes

| scenario | behaviour |
|---|---|
| Serena not running on dev machine | Roadboard works fully; agent cannot resolve nodes; nodes accumulate as `unverified`/`stale`. No degradation of planning/memory features. |
| Node's stable_id path no longer exists | Agent verification flips `resolved_status → broken`. UI shows broken-link badge. User decides to unlink, retarget, or leave for history. |
| Serena and Roadboard disagree on a kind | Roadboard's `kind` is informational only. Serena is authority. Agent may update via a future `update_node_metadata` tool. |

## Roadboard MCP tools related to CodeFlow

See `docs/mcp-tools.md` for the full surface. The 6 CodeFlow tools:

- `link_node`
- `unlink_node`
- `list_node_links`
- `list_links_for_entity`
- `list_stale_nodes`
- `mark_node_status`

All deterministic, all shape-stable, no AI-curated output.
