CREATE TABLE milestones (
    id                 TEXT(36) PRIMARY KEY,
    project_id         TEXT(36) NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title              TEXT     NOT NULL,
    description        TEXT,
    due_date           TEXT,
    status             TEXT     NOT NULL DEFAULT 'planned',
    order_index        INTEGER  NOT NULL DEFAULT 0,
    created_by_user_id TEXT(36) NOT NULL REFERENCES users(id),
    updated_by_user_id TEXT(36) NOT NULL REFERENCES users(id),
    created_at         TEXT     NOT NULL,
    updated_at         TEXT     NOT NULL
);

CREATE INDEX idx_milestones_project_status ON milestones(project_id, status);

CREATE TABLE sprints (
    id                 TEXT(36) PRIMARY KEY,
    project_id         TEXT(36) NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name               TEXT     NOT NULL,
    goal               TEXT,
    start_date         TEXT     NOT NULL,
    end_date           TEXT     NOT NULL,
    status             TEXT     NOT NULL DEFAULT 'planned',
    created_by_user_id TEXT(36) NOT NULL REFERENCES users(id),
    updated_by_user_id TEXT(36) NOT NULL REFERENCES users(id),
    created_at         TEXT     NOT NULL,
    updated_at         TEXT     NOT NULL
);

CREATE INDEX idx_sprints_project_status ON sprints(project_id, status);

CREATE TABLE tasks (
    id                 TEXT(36) PRIMARY KEY,
    project_id         TEXT(36) NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    milestone_id       TEXT(36) REFERENCES milestones(id) ON DELETE SET NULL,
    title              TEXT     NOT NULL,
    description        TEXT,
    status             TEXT     NOT NULL DEFAULT 'todo',
    priority           TEXT     NOT NULL DEFAULT 'medium',
    assignee_user_id   TEXT(36) REFERENCES users(id) ON DELETE SET NULL,
    estimate           TEXT,
    due_date           TEXT,
    created_by_user_id TEXT(36) NOT NULL REFERENCES users(id),
    updated_by_user_id TEXT(36) NOT NULL REFERENCES users(id),
    completed_at       TEXT,
    created_at         TEXT     NOT NULL,
    updated_at         TEXT     NOT NULL
);

CREATE INDEX idx_tasks_project_status    ON tasks(project_id, status);
CREATE INDEX idx_tasks_project_milestone ON tasks(project_id, milestone_id);
CREATE INDEX idx_tasks_assignee_status   ON tasks(assignee_user_id, status);

CREATE TABLE task_dependencies (
    from_task_id    TEXT(36) NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    to_task_id      TEXT(36) NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    dependency_type TEXT     NOT NULL,
    created_at      TEXT     NOT NULL,
    PRIMARY KEY (from_task_id, to_task_id)
);

CREATE TABLE sprint_tasks (
    sprint_id              TEXT(36) NOT NULL REFERENCES sprints(id) ON DELETE CASCADE,
    task_id                TEXT(36) NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    added_at               TEXT     NOT NULL,
    carried_from_sprint_id TEXT(36) REFERENCES sprints(id),
    removed_at             TEXT,
    PRIMARY KEY (sprint_id, task_id)
);

CREATE INDEX idx_sprint_tasks_task_id ON sprint_tasks(task_id);
