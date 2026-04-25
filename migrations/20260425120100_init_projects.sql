CREATE TABLE projects (
    id            TEXT(36) PRIMARY KEY,
    slug          TEXT     NOT NULL,
    name          TEXT     NOT NULL,
    description   TEXT,
    status        TEXT     NOT NULL DEFAULT 'active',
    owner_user_id TEXT(36) NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at    TEXT     NOT NULL,
    updated_at    TEXT     NOT NULL
);

CREATE UNIQUE INDEX idx_projects_owner_slug ON projects(owner_user_id, slug);

CREATE TABLE project_members (
    project_id         TEXT(36) NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id            TEXT(36) NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role               TEXT     NOT NULL,
    granted_by_user_id TEXT(36) NOT NULL REFERENCES users(id),
    created_at         TEXT     NOT NULL,
    PRIMARY KEY (project_id, user_id)
);
