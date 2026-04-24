# Entities — Roadboard 3.0 v1

> 14 first-class entities. PK type: `uuid` v7 stored as `TEXT(36)`. Timestamps:
> `created_at`, `updated_at` implicit on every entity unless noted. All FKs use
> `ON DELETE CASCADE` unless otherwise stated.

## Entity tree at a glance

```
User                                      ProjectMember(role)
  └── owns Projects                            ↑
                                               │
Project ── members: User (via ProjectMember) ──┘
  ├── Milestone                          (WHY axis)
  ├── Sprint                             (WHEN axis)
  ├── Task                               (WHAT)
  │     ├── milestone_id  (N:1)
  │     ├── dependencies  (N:N via TaskDependency)
  │     └── sprints       (N:N via SprintTask)
  ├── MemoryEntry                        (memory)
  ├── Decision                           (memory)
  ├── SessionHandoff                     (memory)
  ├── ArchitectureNode                   (codeflow, minimal — Serena-resolvable)
  └── ArchitectureLink                   (codeflow ↔ {Task|Milestone|Decision|MemoryEntry})

User ── owns ── MCPToken
```

---

## Identity & Access

### User

| field | type | note |
|---|---|---|
| id | uuid v7 | PK |
| username | text | UNIQUE |
| email | text | UNIQUE |
| display_name | text | |
| password_hash | text | argon2id |
| status | enum | `active` \| `disabled` |
| created_at, updated_at | timestamp | |

### MCPToken

| field | type | note |
|---|---|---|
| id | uuid v7 | PK |
| user_id | uuid → User | FK |
| token_name | text | label shown to the owner |
| token_hash | text | one-way hash, never re-readable after creation |
| scopes | json[text] | array of `GrantType` strings |
| status | enum | `active` \| `revoked` |
| expires_at | timestamp? | nullable |
| last_used_at | timestamp? | refreshed on each use |
| revoked_at | timestamp? | |
| created_at | timestamp | |

`GrantType` v1 enum: `project.read`, `project.write`, `task.write`,
`memory.write`, `decision.write`, `codeflow.read`, `codeflow.write`,
`project.admin` (bypass).

---

## Workspace

### Project

| field | type | note |
|---|---|---|
| id | uuid v7 | PK |
| slug | text | UNIQUE per `owner_user_id` |
| name | text | |
| description | text? | |
| status | enum | `active` \| `archived` |
| owner_user_id | uuid → User | FK |
| created_at, updated_at | timestamp | |

### ProjectMember

| field | type | note |
|---|---|---|
| project_id | uuid → Project | PK part 1 |
| user_id | uuid → User | PK part 2 |
| role | enum | `viewer` \| `contributor` \| `admin` \| `owner` |
| granted_by_user_id | uuid → User | |
| created_at | timestamp | |

`role` is coarse on purpose. Fine-grained scope lives on `MCPToken.scopes` —
**humans have roles, agents have scopes**.

---

## Planning

### Milestone

| field | type | note |
|---|---|---|
| id | uuid v7 | PK |
| project_id | uuid → Project | FK |
| title | text | |
| description | text? | |
| due_date | date? | nullable |
| status | enum | `planned` \| `in_progress` \| `done` \| `cancelled` |
| order_index | int | manual sort |
| created_by_user_id | uuid → User | |
| updated_by_user_id | uuid → User | |
| created_at, updated_at | timestamp | |

### Sprint

| field | type | note |
|---|---|---|
| id | uuid v7 | PK |
| project_id | uuid → Project | FK |
| name | text | e.g. `"Sprint 12 — Bug bash"` |
| goal | text? | the synthetic "why" |
| start_date | date | |
| end_date | date | |
| status | enum | `planned` \| `active` \| `closed` |
| created_by_user_id | uuid → User | |
| updated_by_user_id | uuid → User | |
| created_at, updated_at | timestamp | |

INDEX: `(project_id, status)`.
At most **one** sprint per project may have `status=active` — enforced at the
application layer (not via DB constraint, to allow flexibility during transitions).

### Task

| field | type | note |
|---|---|---|
| id | uuid v7 | PK |
| project_id | uuid → Project | FK |
| milestone_id | uuid? → Milestone | nullable for "uncategorized" |
| title | text | |
| description | text? | |
| status | enum | `todo` \| `in_progress` \| `blocked` \| `done` \| `cancelled` |
| priority | enum | `low` \| `medium` \| `high` \| `urgent` |
| assignee_user_id | uuid? → User | |
| estimate | text? | free string (`"3h"`, `"5pt"`, `"S"`) — no unit imposed |
| due_date | date? | |
| created_by_user_id | uuid → User | |
| updated_by_user_id | uuid → User | |
| completed_at | timestamp? | populated when `status=done` |
| created_at, updated_at | timestamp | |

INDEX: `(project_id, status)`, `(project_id, milestone_id)`,
`(assignee_user_id, status)`.

### TaskDependency

| field | type | note |
|---|---|---|
| from_task_id | uuid → Task | PK part 1 |
| to_task_id | uuid → Task | PK part 2 |
| dependency_type | enum | `blocks` \| `relates` \| `duplicates` |
| created_at | timestamp | |

PK composite. Self-loop (`from_task_id = to_task_id`) blocked at the
application layer.

### SprintTask

| field | type | note |
|---|---|---|
| sprint_id | uuid → Sprint | PK part 1 |
| task_id | uuid → Task | PK part 2 |
| added_at | timestamp | |
| carried_from_sprint_id | uuid? → Sprint | tracks carry-over |
| removed_at | timestamp? | soft remove from sprint without deleting the task |

INDEX: `(task_id)` to support "in which sprints does this task live" queries.

---

## Memory & Knowledge

### MemoryEntry

| field | type | note |
|---|---|---|
| id | uuid v7 | PK |
| project_id | uuid → Project | FK |
| category | enum | see below |
| title | text | |
| body | text | |
| status | enum | `active` \| `archived` |
| durability | enum | `transient` \| `working` \| `durable` |
| source_type | enum | `manual` \| `agent` \| `handoff` |
| source_ref | text? | free identifier (sessionId, commit hash, etc.) |
| created_by_user_id | uuid → User | |
| created_at, updated_at | timestamp | |

`category` enum: `activity`, `next_step`, `decision_context`, `architecture`,
`issue`, `learning`, `operational_note`, `open_question`.

INDEX: `(project_id, category, status)`.
Mirrored into `searchable` FTS5 virtual table on `(title, body)` via triggers.

### Decision

| field | type | note |
|---|---|---|
| id | uuid v7 | PK |
| project_id | uuid → Project | FK |
| title | text | |
| summary | text | the decision in 1-3 sentences |
| rationale | text | the why |
| impact_level | enum | `low` \| `medium` \| `high` \| `critical` |
| status | enum | `proposed` \| `accepted` \| `superseded` \| `rejected` |
| resolved_at | timestamp? | populated at terminal status |
| superseded_by_decision_id | uuid? → Decision | self-ref FK |
| created_by_user_id, updated_by_user_id | uuid → User | |
| created_at, updated_at | timestamp | |

INDEX: `(project_id, status)`.
Mirrored into `searchable` on `(title, summary, rationale)`.

### SessionHandoff

| field | type | note |
|---|---|---|
| id | uuid v7 | PK |
| project_id | uuid → Project | FK |
| user_id | uuid → User | author |
| title | text | |
| summary | text | |
| what_was_done | text | |
| next_steps | text | |
| blockers | text? | |
| created_at | timestamp | |

Immutable post-creation (no `updated_at`).
Mirrored into `searchable` on `(title, summary, what_was_done, next_steps)`.

---

## CodeFlow (minimal — Serena federation)

### ArchitectureNode

| field | type | note |
|---|---|---|
| id | uuid v7 | PK (internal Roadboard id) |
| project_id | uuid → Project | FK |
| stable_id | text | Serena-resolvable identifier — see `docs/codeflow-verification.md` |
| kind | text | LSP symbol kind (free string: function, struct, trait, module, file…) |
| name | text | symbol name |
| path | text | file path relative to project root, POSIX-normalised |
| last_seen_at | timestamp | refreshed every time the node is "touched" |
| resolved_status | enum | `unverified` \| `resolved` \| `stale` \| `broken` |
| created_at, updated_at | timestamp | |

UNIQUE: `(project_id, stable_id)`.
INDEX: `(project_id, path)`, `(project_id, resolved_status)`.

**Note**: no `description` field. Free-form descriptive content lives in a
linked `MemoryEntry` of category `architecture`. Single source of truth for
free text.

### ArchitectureLink

| field | type | note |
|---|---|---|
| id | uuid v7 | PK |
| project_id | uuid → Project | FK |
| node_id | uuid → ArchitectureNode | FK |
| entity_type | enum | `task` \| `milestone` \| `decision` \| `memory_entry` |
| entity_id | uuid | polymorphic — application-level validation |
| link_type | enum | `implements` \| `modifies` \| `fixes` \| `addresses` \| `motivates` \| `constrains` \| `delivers` \| `describes` \| `warns_about` \| `relates_to` |
| note | text? | |
| created_by_user_id | uuid → User | |
| created_at | timestamp | |

UNIQUE: `(node_id, entity_type, entity_id, link_type)` — no duplicates.
INDEX: `(project_id, entity_type, entity_id)` for "given a Task, what nodes are linked".

---

## Conventions

- **Stable identifier** (`ArchitectureNode.stable_id`): formalised in
  `docs/codeflow-verification.md`. Format: `<posix-relative-path>::<Symbol::Path::Qualified>`.
  Roadboard does not validate it — Serena is authority.
- **Soft delete**: not used. Hard delete with `CASCADE`. The `archived` status
  covers "no longer relevant but keep the record".
- **Audit minimum**: `created_by_user_id` / `updated_by_user_id` on
  planning + memory entities. No separate `ActivityEvent` stream in v1.
- **Polymorphism**: `ArchitectureLink.entity_id` is polymorphic via
  `entity_type`. Existence validation is application-level (no DB-level FK).
  Same trade-off as RB.v2 — right cost/benefit for v1.

---

## What is NOT here (explicit deferrals from RB.v2)

| RB.v2 entity | v1 status | Reason |
|---|---|---|
| Phase | removed | Replaced by orthogonal Milestone + Sprint |
| Team / TeamMembership | deferred | Multi-user works without teams day-0 |
| FileReference | removed | Superseded by ArchitectureNode |
| AgentSession | out | Low value, AI-centric overhead |
| ContextBundle | out | Violates "no AI bundle on read" invariant |
| ActivityEvent | deferred | Audit covered by `created_by/updated_by` |
| SyncRecord | removed | local-sync-bridge dropped in v3 |
| CodeRepository | removed | Serena owns the repo, no need to duplicate |
| ArchitectureSnapshot | removed | Serena passthrough, no historicization |
| ArchitectureEdge | removed | Edges = LSP live query |
| ArchitectureAnnotation | removed | Folded into linked MemoryEntry |
| ImpactAnalysis | removed | On-demand via Serena, no cache |

**14 entities in v1 vs 28+ in v2** — half the surface, full coverage.
