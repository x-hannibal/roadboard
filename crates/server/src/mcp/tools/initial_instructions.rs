use serde_json::{json, Value};

pub async fn run() -> Result<Value, super::super::error::ToolError> {
    Ok(json!({
        "schema_version": 1,
        "protocol_version": "roadboard-3.0",
        "operational_rules": [
            "Code intelligence federation: query Serena MCP for symbol resolution.",
            "Stable_id format: <posix-relative-path>::<Symbol::Path::Qualified>.",
            "Verify stale nodes at session start: list_stale_nodes → serena.find_symbol → mark_node_status.",
            "Persist meaningful findings via create_memory_entry without asking permission.",
            "End sessions with create_handoff covering what_was_done + next_steps + blockers."
        ],
        "tool_registry": tool_registry(),
        "error_code_summary": {
            "NOT_FOUND": "Entity does not exist or is not accessible to this token",
            "FORBIDDEN": "Token scope insufficient for this operation",
            "VALIDATION_ERROR": "Input failed validation; see details for field errors",
            "CONFLICT": "Operation would violate a uniqueness or state constraint",
            "INTERNAL_ERROR": "Unexpected server error; report with requestId"
        }
    }))
}

fn tool_registry() -> Value {
    json!([
        { "name": "initial_instructions",      "scope": "-",               "summary": "Returns operational rules, tool registry, and error code summary", "status": "available" },
        { "name": "get_task_briefing",          "scope": "project.read",    "summary": "Full fact pack for a task (task + milestone + sprint + linked nodes)", "status": "not_implemented" },
        { "name": "list_projects",             "scope": "project.read",    "summary": "List projects accessible to the caller", "status": "available" },
        { "name": "get_project",               "scope": "project.read",    "summary": "Project detail with milestone/sprint/task/member counts", "status": "available" },
        { "name": "create_project",            "scope": "project.admin",   "summary": "Create a new project and auto-join caller as Owner", "status": "available" },
        { "name": "list_milestones",           "scope": "project.read",    "summary": "List milestones for a project with optional status filter", "status": "available" },
        { "name": "create_milestone",          "scope": "project.write",   "summary": "Create a milestone in a project", "status": "available" },
        { "name": "list_sprints",              "scope": "project.read",    "summary": "List sprints for a project with optional status filter", "status": "available" },
        { "name": "get_active_sprint",         "scope": "project.read",    "summary": "Return the active sprint for a project, or null", "status": "available" },
        { "name": "create_sprint",             "scope": "project.write",   "summary": "Create a sprint in a project", "status": "available" },
        { "name": "add_task_to_sprint",        "scope": "task.write",      "summary": "Link a task to a sprint", "status": "available" },
        { "name": "remove_task_from_sprint",   "scope": "task.write",      "summary": "Soft-remove a task from a sprint", "status": "available" },
        { "name": "list_tasks",                "scope": "project.read",    "summary": "List tasks with optional filters", "status": "available" },
        { "name": "get_task",                  "scope": "project.read",    "summary": "Get a single task by ID", "status": "available" },
        { "name": "create_task",               "scope": "task.write",      "summary": "Create a task in a project", "status": "available" },
        { "name": "update_task",               "scope": "task.write",      "summary": "Update task fields (partial update)", "status": "available" },
        { "name": "update_task_status",        "scope": "task.write",      "summary": "Transition task status", "status": "available" },
        { "name": "search_memory",             "scope": "project.read",    "summary": "FTS5 search across memory entries", "status": "not_implemented" },
        { "name": "list_memory",               "scope": "project.read",    "summary": "List memory entries with optional filters", "status": "not_implemented" },
        { "name": "get_memory_entry",          "scope": "project.read",    "summary": "Get a single memory entry by ID", "status": "not_implemented" },
        { "name": "create_memory_entry",       "scope": "memory.write",    "summary": "Create a memory entry", "status": "not_implemented" },
        { "name": "update_memory_entry",       "scope": "memory.write",    "summary": "Update a memory entry (archive via status)", "status": "not_implemented" },
        { "name": "list_decisions",            "scope": "project.read",    "summary": "List decisions with optional filters", "status": "not_implemented" },
        { "name": "get_decision",              "scope": "project.read",    "summary": "Get a decision with linked nodes/tasks/memory", "status": "not_implemented" },
        { "name": "create_decision",           "scope": "decision.write",  "summary": "Create a decision (status defaults to proposed)", "status": "not_implemented" },
        { "name": "update_decision",           "scope": "decision.write",  "summary": "Update decision status or supersede chain", "status": "not_implemented" },
        { "name": "create_handoff",            "scope": "memory.write",    "summary": "Create an immutable session handoff", "status": "not_implemented" },
        { "name": "link_node",                 "scope": "codeflow.write",  "summary": "Link a Serena node to an operational entity (find-or-create)", "status": "not_implemented" },
        { "name": "unlink_node",               "scope": "codeflow.write",  "summary": "Delete a node link", "status": "not_implemented" },
        { "name": "list_node_links",           "scope": "codeflow.read",   "summary": "List entities linked to a stable_id node", "status": "not_implemented" },
        { "name": "list_links_for_entity",     "scope": "codeflow.read",   "summary": "List nodes linked to an operational entity", "status": "not_implemented" },
        { "name": "list_stale_nodes",          "scope": "codeflow.read",   "summary": "List nodes due for verification", "status": "not_implemented" },
        { "name": "mark_node_status",          "scope": "codeflow.write",  "summary": "Update resolved status of a node", "status": "not_implemented" },
        { "name": "search",                    "scope": "project.read",    "summary": "FTS5 search across memory, decisions, handoffs, tasks", "status": "not_implemented" }
    ])
}
