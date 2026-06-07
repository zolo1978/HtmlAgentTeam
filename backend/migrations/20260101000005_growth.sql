-- Migration 005: 成长体系
CREATE TABLE agent_growth (
  id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id            UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE UNIQUE,
  level               VARCHAR(10) NOT NULL DEFAULT 'L1',
  growth_score        NUMERIC(10,2) NOT NULL DEFAULT 0,
  task_score          NUMERIC(10,2) NOT NULL DEFAULT 0,
  skill_score         NUMERIC(10,2) NOT NULL DEFAULT 0,
  reflection_score    NUMERIC(10,2) NOT NULL DEFAULT 0,
  memory_score        NUMERIC(10,2) NOT NULL DEFAULT 0,
  completed_tasks     INTEGER NOT NULL DEFAULT 0,
  reflection_count    INTEGER NOT NULL DEFAULT 0,
  updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE growth_events (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id     UUID        NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  event_type   VARCHAR(50) NOT NULL,
  score_delta  NUMERIC(8,2) NOT NULL DEFAULT 0,
  metadata     JSONB       NOT NULL DEFAULT '{}',
  created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_growth_events_agent_id ON growth_events(agent_id);
