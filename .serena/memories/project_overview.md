# Roadboard — Project Overview (snapshot)

## Status
Branch `rb-rust`. **Greenfield Rust rewrite** of the previous Roadboard 2.0
(TypeScript/NestJS monorepo). No PLAN.md / docs yet — project is in
**Design Briefing mode** (Architect role active).

## Reference
A full read-only snapshot of the previous implementation lives at `RB.v2/`
(gitignored). It is the "sample for analysis" — not code to port.

Key RB.v2 artifacts to consult when briefing design questions:
- `RB.v2/README.md` — elevator pitch + 23-tool MCP surface
- `RB.v2/docs/planning/vision.md` — product vision (still valid)
- `RB.v2/docs/planning/entities.md` — data model (Projects, Phases,
  Milestones, Tasks, MemoryEntry, Decision, SessionHandoff, User, Team,
  ProjectGrant, MCPToken, ActivityEvent)
- `RB.v2/docs/design/codeflow.md` — Wave 5 architecture graph concept
  (6 Prisma models, MCP tools `get_architecture_map`, `get_node_context`,
  `get_change_impact`, `get_architecture_snapshot`)
- `RB.v2/ROADMAP.md` — waves 1–3 complete, waves 4–5 partially shipped

## Evolution direction (user-stated)
- Stack → **Rust + Tauri + SQLite** (graph DB undecided)
- Philosophy → shift load from AI to **deterministic logic**;
  delegate language-structural queries to **LSP, ideally via Serena**
  as context server
- Deploy topology → **dual**:
  - Local-only: Tauri app (agent + UI in-process)
  - Local + remote: local agent + Axum server serving the *same* Next.js UI
- Scope label → **EVOLUTION** (not a port, not a reset)

## Anti-goals (from RB.v2 vision, still valid)
- Not a Jira/Linear replacement
- No opaque autonomous agent behavior
- No RAG/semantic infra in v1
- Not tied to a single LLM vendor

## Session workflow
Active protocol = root `CLAUDE.md` (Architect/Worker split, tasks/todo/run/done).
The `CLAUDE.md` inside `RB.v2/` is the old protocol — does not apply.
