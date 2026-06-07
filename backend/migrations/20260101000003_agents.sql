-- Migration 003: Agent 核心
CREATE TABLE agents (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id      UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  owner_id    UUID        NOT NULL REFERENCES users(id),
  name        VARCHAR(255) NOT NULL,
  role        VARCHAR(255) NOT NULL,
  description TEXT        NOT NULL DEFAULT '',
  status      VARCHAR(50)  NOT NULL DEFAULT 'created',
  settings    JSONB        NOT NULL DEFAULT '{}',
  created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
  updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_agents_org_id   ON agents(org_id);
CREATE INDEX idx_agents_status   ON agents(status);
CREATE INDEX idx_agents_owner_id ON agents(owner_id);
