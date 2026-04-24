# CodeFlow node verification — Roadboard 3.0 v1

> The canonical workflow an agent runs to keep `ArchitectureNode` records
> aligned with the actual codebase. Roadboard server has no live access to
> code; the agent — which holds both the Roadboard MCP and the Serena MCP —
> performs the reconciliation.

## When to run

- **At session start**: discover stale or unverified nodes that may have
  drifted since the last session.
- **Before relying on a node's resolved status** in a decision/task creation
  flow.
- **After significant code changes** (renames, refactors, file moves) when
  the agent knows it has just modified the codebase.
- **On user request**: a "verify nodes" command in the UI calls the same
  underlying tools.

## The workflow

```
┌────────────────────────────────────────────────────────────────────────┐
│ 1. List candidates                                                     │
│                                                                        │
│    agent → roadboard.list_stale_nodes(                                 │
│              projectId,                                                │
│              maxAgeDays = 7,                                           │
│              includeUnverified = true                                  │
│            )                                                           │
│                                                                        │
│         ← {                                                            │
│             items: [                                                   │
│               {                                                        │
│                 stable_id: "crates/core/src/auth.rs::AuthService",     │
│                 kind: "struct",                                        │
│                 name: "AuthService",                                   │
│                 path: "crates/core/src/auth.rs",                       │
│                 last_seen_at: "...",                                   │
│                 resolved_status: "stale"                               │
│               },                                                       │
│               ...                                                      │
│             ],                                                         │
│             next_cursor: null                                          │
│           }                                                            │
└────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌────────────────────────────────────────────────────────────────────────┐
│ 2. For each node, query Serena                                         │
│                                                                        │
│    Split stable_id at the first '::' into:                             │
│      - path:      everything before  (e.g. "crates/core/src/auth.rs")  │
│      - qualifier: everything after   (e.g. "AuthService")              │
│    If no '::' is present, qualifier is empty (file-level node).        │
│                                                                        │
│    agent → serena.find_symbol(                                         │
│              name_path = qualifier,                                    │
│              relative_path = path,                                     │
│              include_body = false                                      │
│            )                                                           │
│                                                                        │
│    Outcomes:                                                           │
│      - symbol resolves                  → status = "resolved"          │
│      - empty / not found                → status = "broken"            │
│      - ambiguous (multiple matches)     → status = "broken" (manual    │
│                                          re-link required)             │
└────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌────────────────────────────────────────────────────────────────────────┐
│ 3. Report back to Roadboard                                            │
│                                                                        │
│    agent → roadboard.mark_node_status(                                 │
│              projectId,                                                │
│              stable_id,                                                │
│              status                                                    │
│            )                                                           │
│                                                                        │
│         ← updated node record                                          │
└────────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌────────────────────────────────────────────────────────────────────────┐
│ 4. (Optional) clean up broken links                                    │
│                                                                        │
│    For nodes that just transitioned to "broken" and whose links are    │
│    irrecoverable, the agent — with user confirmation in interactive    │
│    sessions, autonomously in batch sessions — calls:                   │
│                                                                        │
│    agent → roadboard.list_node_links(projectId, stable_id)             │
│         ← list of links                                                │
│                                                                        │
│    agent → roadboard.unlink_node(linkId)   per link to remove          │
└────────────────────────────────────────────────────────────────────────┘
```

## Splitting `stable_id`

```python
def split_stable_id(stable_id: str) -> tuple[str, str]:
    if "::" in stable_id:
        path, _, qualifier = stable_id.partition("::")
        return path, qualifier
    return stable_id, ""
```

`partition("::")` (split on the first occurrence) is correct — Symbol
qualifiers can contain further `::` separators (`Module::Type::method`),
which Serena's `name_path` handles natively.

## Status transition rules

The agent should respect the state machine in
`docs/serena-integration.md` § Resolved-status state machine. Key invariant:
**`mark_node_status` always succeeds** for legal target statuses (`resolved`
or `broken`); `stale` is set only by Roadboard via TTL and `unverified` is
set only at create time. Trying to set `stale` or `unverified` returns
`INVALID_STATE_TRANSITION`.

## Performance considerations

- Each node verification is one Serena call. Process in small batches
  (50 at a time) to keep individual tool-call latency bounded.
- For very large projects (>1000 architecture nodes), `list_stale_nodes`
  pages naturally — verify one page, then fetch next.
- Roadboard's `mark_node_status` updates `last_seen_at = now()` as a
  side effect, so re-verified nodes "reset" the TTL.

## Error handling

| Outcome of `serena.find_symbol` | Action |
|---|---|
| Resolves to exactly 1 symbol | `mark_node_status(stable_id, "resolved")` |
| Returns empty | `mark_node_status(stable_id, "broken")` |
| Returns multiple matches (ambiguous) | `mark_node_status(stable_id, "broken")` and prompt user/agent to re-link with disambiguating qualifier |
| Serena unreachable | Skip — leave status as is. Surface the failure to the user/agent log. |
| Roadboard unreachable | Halt verification; report. Do not retry indefinitely. |

## Out of scope (for v1)

- Auto-rename detection (find a renamed symbol and update the stable_id).
  Possible v2 enhancement using Serena's symbol-graph + git history.
- Bulk reconciliation triggered by file watchers. v1 is on-demand only.
- UI-side proxy of the verification workflow (Tauri-only Wave 2 feature).
