# MCP error codes — Roadboard 3.0 v1

> Stable contract. Codes within `protocol_version` are field-additive: never
> renamed, never removed. New codes may be added.

## Envelope

Every error returns:

```jsonc
{
  "error": {
    "code": "SNAKE_CASE_CODE",
    "message": "human-readable summary",
    "details": { /* optional, code-specific */ }
  }
}
```

Successful responses never include the `error` key.

## Code reference

### Authentication & authorisation

| Code | HTTP | When | `details` |
|---|---|---|---|
| `UNAUTHENTICATED` | 401 | No bearer token, malformed token, expired token | `{ reason: "missing_token" \| "invalid_format" \| "expired" \| "revoked" }` |
| `FORBIDDEN` | 403 | Token authenticated but lacks required scope | `{ required_scope: "task.write", token_scopes: ["project.read"] }` |
| `PROJECT_ACCESS_DENIED` | 403 | Token has scope but the user has no `ProjectMember` row for the target project | `{ project_id: "..." }` |

### Lookup

| Code | HTTP | When | `details` |
|---|---|---|---|
| `NOT_FOUND` | 404 | Entity does not exist or is not accessible | `{ entity_type: "task", entity_id: "..." }` |
| `STALE_RESOURCE` | 409 | Entity exists but was modified since last read (when version conflict checking is added — v2) | `{ entity_type, entity_id, current_version, supplied_version }` |

### Validation

| Code | HTTP | When | `details` |
|---|---|---|---|
| `VALIDATION_ERROR` | 400 | Input field constraints violated | `{ fields: { "title": "must not be empty", "due_date": "..." } }` |
| `INVALID_ENUM` | 400 | Enum field received a value not in the allowed set | `{ field: "status", allowed: ["todo", "in_progress", ...], received: "pending" }` |
| `INVALID_REFERENCE` | 400 | A referenced entity does not exist (foreign key violation, cross-project leak attempt) | `{ field: "milestone_id", value: "..." }` |
| `INVALID_CURSOR` | 400 | Pagination cursor is malformed or no longer valid | `{}` |

### State

| Code | HTTP | When | `details` |
|---|---|---|---|
| `CONFLICT` | 409 | Operation would violate uniqueness or state constraint | `{ constraint: "unique_slug", existing_id: "..." }` |
| `INVALID_STATE_TRANSITION` | 409 | Status transition not allowed by the entity's state machine | `{ entity_type, current: "done", attempted: "in_progress" }` |
| `IMMUTABLE_RESOURCE` | 409 | Attempt to modify a resource that is immutable post-creation (SessionHandoff) | `{ entity_type, entity_id }` |

### Capacity / rate

| Code | HTTP | When | `details` |
|---|---|---|---|
| `RATE_LIMITED` | 429 | Token exceeded request quota | `{ retry_after_seconds: 30 }` |
| `PAYLOAD_TOO_LARGE` | 413 | Request body exceeds limit (e.g. memory body > 1 MB) | `{ field: "body", max_bytes: 1048576, received_bytes: 2300000 }` |

### Server

| Code | HTTP | When | `details` |
|---|---|---|---|
| `INTERNAL_ERROR` | 500 | Unhandled error | `{ request_id: "..." }` (always include `request_id` for support correlation) |
| `STORAGE_UNAVAILABLE` | 503 | DB connection pool exhausted, file lock timeout | `{ retry_after_seconds: 5 }` |

## Per-tool special cases

A handful of tools have semantics that warrant dedicated codes:

| Tool | Code | When |
|---|---|---|
| `add_task_to_sprint` | `TASK_ALREADY_IN_SPRINT` | Active link already exists between sprint and task |
| `add_task_to_sprint` | `TASK_PROJECT_MISMATCH` | Task and sprint belong to different projects |
| `mark_node_status` | `INVALID_STATE_TRANSITION` | e.g. trying to move from a non-existent status |
| `update_decision` | `SUPERSEDE_CYCLE` | `superseded_by_decision_id` would create a cycle |
| `link_node` | `LINK_ENTITY_NOT_FOUND` | Polymorphic `entity_id` does not exist for the given `entity_type` |
| `link_node` | `DUPLICATE_LINK` | UNIQUE on `(node_id, entity_type, entity_id, link_type)` violated |
| `create_handoff` | `EMPTY_HANDOFF` | Both `summary` and `next_steps` are empty |

## Client guidance

Stable codes mean clients can branch programmatically:

```typescript
const result = await rb.create_task({...});
if ("error" in result) {
  switch (result.error.code) {
    case "VALIDATION_ERROR":
      return showFieldErrors(result.error.details.fields);
    case "PROJECT_ACCESS_DENIED":
      return promptForReauth();
    default:
      return reportToTelemetry(result.error);
  }
}
```

Always log `details.request_id` (when present) — server-side logs are
correlated by it.
