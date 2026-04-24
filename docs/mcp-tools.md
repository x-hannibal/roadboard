# MCP tools — Roadboard 3.0 v1

> 34 tools total, organised in 11 groups. Naming is `verb_noun` snake_case.
> All responses are deterministic JSON, shape-stable, versioned via
> `schema_version`. No prose, no LLM-curated fields.

## Conventions

- **Pagination**: every `list_*` accepts `cursor?: string` (opaque,
  base64-encoded `{ last_id, last_sort_value }`) + `limit?: number`
  (default 50, max 200). Response includes `next_cursor: string | null`.
- **Errors**: shape `{ error: { code: SNAKE_CASE, message: string, details?: object } }`. Codes are stable — see `docs/mcp-error-codes.md`.
- **Scope enforcement**: each tool declares the minimum required `GrantType`
  on the calling token. `project.admin` bypasses all checks.
- **Project scoping**: every tool except `initial_instructions`, `list_projects`, `get_project`, and `create_project` operates within a single project. The project is determined by the first parameter (`projectId`) or by lookup of the entity referenced.
- **Backward compatibility**: within a `protocol_version`, fields are
  field-additive — never renamed, never removed. Breaking change → bump
  `protocol_version`.

## Tool census

### Bootstrap

| # | Tool | Scope | Returns |
|---|---|---|---|
| 1 | `initial_instructions()` | — | `{ schema_version, protocol_version, operational_rules, tool_registry, error_code_summary }` |

### Briefing

| # | Tool | Scope | Returns |
|---|---|---|---|
| 2 | `get_task_briefing(taskId)` | `project.read` | full fact pack — see shape below |

### Project

| # | Tool | Scope | Returns |
|---|---|---|---|
| 3 | `list_projects(cursor?, limit?)` | `project.read` | accessible projects |
| 4 | `get_project(projectId)` | `project.read` | project + counts |
| 5 | `create_project(name, slug?, description?)` | `project.admin` | new project |

### Milestone

| # | Tool | Scope | Returns |
|---|---|---|---|
| 6 | `list_milestones(projectId, status?, cursor?, limit?)` | `project.read` | milestones |
| 7 | `create_milestone(projectId, title, dueDate?, description?, orderIndex?)` | `project.write` | new milestone |

### Sprint

| # | Tool | Scope | Returns |
|---|---|---|---|
| 8 | `list_sprints(projectId, status?, cursor?, limit?)` | `project.read` | sprints |
| 9 | `get_active_sprint(projectId)` | `project.read` | the one with `status=active`, or `null` |
| 10 | `create_sprint(projectId, name, startDate, endDate, goal?)` | `project.write` | new sprint |
| 11 | `add_task_to_sprint(sprintId, taskId, carriedFromSprintId?)` | `task.write` | sprint_task link |
| 12 | `remove_task_from_sprint(sprintId, taskId)` | `task.write` | soft-removed link |

### Task

| # | Tool | Scope | Returns |
|---|---|---|---|
| 13 | `list_tasks(projectId, status?, milestoneId?, sprintId?, assigneeUserId?, cursor?, limit?)` | `project.read` | filtered tasks |
| 14 | `get_task(taskId)` | `project.read` | task + linked nodes/decisions/memory in compact form |
| 15 | `create_task(projectId, title, milestoneId?, sprintId?, description?, priority?, assigneeUserId?, dueDate?, estimate?)` | `task.write` | new task |
| 16 | `update_task(taskId, title?, description?, milestoneId?, priority?, assigneeUserId?, dueDate?, estimate?)` | `task.write` | updated task |
| 17 | `update_task_status(taskId, status, completionReport?)` | `task.write` | updated task |

### Memory

| # | Tool | Scope | Returns |
|---|---|---|---|
| 18 | `search_memory(projectId, query, category?, durability?, cursor?, limit?)` | `project.read` | FTS5-ranked hits |
| 19 | `list_memory(projectId, category?, durability?, status?, cursor?, limit?)` | `project.read` | non-query listing |
| 20 | `get_memory_entry(memoryId)` | `project.read` | full entry |
| 21 | `create_memory_entry(projectId, category, title, body, durability, sourceType?, sourceRef?)` | `memory.write` | new entry |
| 22 | `update_memory_entry(memoryId, title?, body?, category?, durability?, status?)` | `memory.write` | updated entry (archive via `status: archived`) |

### Decision

| # | Tool | Scope | Returns |
|---|---|---|---|
| 23 | `list_decisions(projectId, status?, impactLevel?, cursor?, limit?)` | `project.read` | decisions |
| 24 | `get_decision(decisionId)` | `project.read` | decision + linked nodes/tasks/memory |
| 25 | `create_decision(projectId, title, summary, rationale, impactLevel)` | `decision.write` | new decision (status default `proposed`) |
| 26 | `update_decision(decisionId, status?, supersededByDecisionId?, title?, summary?, rationale?, impactLevel?)` | `decision.write` | updated decision |

### Handoff

| # | Tool | Scope | Returns |
|---|---|---|---|
| 27 | `create_handoff(projectId, title, summary, whatWasDone, nextSteps, blockers?)` | `memory.write` | new immutable handoff |

### CodeFlow

| # | Tool | Scope | Returns |
|---|---|---|---|
| 28 | `link_node(projectId, stableId, entityType, entityId, linkType, note?)` | `codeflow.write` | link (find-or-create node) |
| 29 | `unlink_node(linkId)` | `codeflow.write` | `{ deleted: true }` |
| 30 | `list_node_links(projectId, stableId)` | `codeflow.read` | entities linked to that node |
| 31 | `list_links_for_entity(projectId, entityType, entityId)` | `codeflow.read` | nodes linked to that entity |
| 32 | `list_stale_nodes(projectId, maxAgeDays?, includeUnverified?)` | `codeflow.read` | nodes due verification |
| 33 | `mark_node_status(projectId, stableId, status)` | `codeflow.write` | updated node |

### Cross-entity search

| # | Tool | Scope | Returns |
|---|---|---|---|
| 34 | `search(projectId, query, types?, cursor?, limit?)` | `project.read` | FTS5 hits across `memory`, `decision`, `handoff`, `task` |

## Key response shapes

### `initial_instructions` response

```jsonc
{
  "schema_version": 1,
  "protocol_version": "roadboard-3.0",
  "operational_rules": [
    "Code intelligence federation: query Serena MCP for symbol resolution.",
    "Stable_id format: <posix-relative-path>::<Symbol::Path::Qualified>.",
    "Verify stale nodes at session start: list_stale_nodes → serena.find_symbol → mark_node_status.",
    "Persist meaningful findings via create_memory_entry without asking permission.",
    "End sessions with create_handoff covering what_was_done + next_steps + blockers."
  ],
  "tool_registry": [
    { "name": "list_projects", "scope": "project.read", "summary": "List accessible projects" },
    /* … all 34 tools with one-line summaries … */
  ],
  "error_code_summary": {
    "NOT_FOUND": "Entity does not exist or is not accessible to this token",
    "FORBIDDEN": "Token scope insufficient for this operation",
    "VALIDATION_ERROR": "Input failed validation; see details for field errors",
    "CONFLICT": "Operation would violate a uniqueness or state constraint",
    "INTERNAL_ERROR": "Unexpected server error; report with requestId"
  }
}
```

### `get_task_briefing(taskId)` response

```jsonc
{
  "schema_version": 1,
  "task": {
    "id": "...",
    "title": "...",
    "description": "...",
    "status": "in_progress",
    "priority": "high",
    "assignee_user_id": "...",
    "due_date": "2026-05-10",
    "estimate": "3d",
    "created_at": "...",
    "updated_at": "..."
  },
  "milestone": {
    "id": "...",
    "title": "...",
    "due_date": "2026-06-01",
    "status": "in_progress"
  } /* or null */,
  "current_sprint": {
    "id": "...",
    "name": "Sprint 12",
    "start_date": "...",
    "end_date": "...",
    "goal": "..."
  } /* or null */,
  "linked": {
    "nodes": [
      {
        "stable_id": "crates/core/src/auth.rs::AuthService::verify_token",
        "kind": "function",
        "name": "verify_token",
        "path": "crates/core/src/auth.rs",
        "link_type": "modifies",
        "resolved_status": "resolved"
      }
    ],
    "decisions": [
      { "id": "...", "title": "...", "status": "accepted", "link_type": "addresses" }
    ],
    "memory": [
      { "id": "...", "category": "issue", "title": "...", "snippet": "...", "link_type": "describes" }
    ]
  },
  "active_handoff": {
    "id": "...",
    "title": "...",
    "next_steps": "...",
    "blockers": "...",
    "created_at": "..."
  } /* or null */,
  "hints": {
    "code_intelligence_provider": "serena",
    "node_resolver_tool": "serena.find_symbol",
    "node_id_format": "path/to/file::qualified::name"
  }
}
```

### Generic list response shape

```jsonc
{
  "schema_version": 1,
  "items": [ /* … */ ],
  "next_cursor": "eyJsYXN0X2lkIjogIi4uLiIsICJsYXN0X3NvcnRfdmFsdWUiOiAiLi4uIn0=" /* or null */
}
```

### Error response shape

```jsonc
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Input failed validation",
    "details": {
      "fields": {
        "title": "must not be empty",
        "due_date": "must be ISO-8601 date"
      },
      "request_id": "01JAB123…"
    }
  }
}
```

## Tools removed compared to RB.v2

| RB.v2 tool | v3 status | Reason |
|---|---|---|
| `prepare_project_summary` | removed | Violates "no AI bundle on read" |
| `prepare_task_context` | renamed `get_task_briefing` | Confirmed deterministic shape |
| `get_project_changelog` | removed | Audit log out of scope v1 |
| `list_phases`, `create_phase`, `update_phase` | removed | Phase entity does not exist |
| `get_architecture_map` | removed | Structure = Serena |
| `get_node_context` | removed | Replaced by `list_node_links` + `list_links_for_entity` + Serena |
| `list_active_tasks`, `list_recent_decisions`, `get_project_memory` | generalised into `list_*` with filters | |
| `update_milestone`, `update_sprint`, `set_task_dependencies`, `list_handoffs`, `get_latest_handoff` | deferred to v2 | UI handles in v1; agents rarely need |

## Versioning policy

- `schema_version` (integer in every response): bumped when a response shape
  changes in a non-additive way for that tool.
- `protocol_version` (string in `initial_instructions`): bumped when a
  cross-cutting breaking change happens (tool removed, scope renamed, etc.).
- Pre-1.0: breaking changes accepted with explicit `CHANGELOG.md` mention.
- Post-1.0: strict additive within `protocol_version`.
