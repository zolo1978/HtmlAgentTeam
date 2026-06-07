-- Migration 004: 记忆（向量存储）
CREATE TABLE memories (
  id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id          UUID        NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  org_id            UUID        NOT NULL REFERENCES organizations(id),
  memory_type       VARCHAR(20) NOT NULL,
  source            VARCHAR(50) NOT NULL,
  content           TEXT        NOT NULL,
  compressed_content TEXT,
  original_tokens   INTEGER,
  compressed_tokens INTEGER,
  compression_ratio NUMERIC(4,2),
  embedding         vector(1536),
  metadata          JSONB       NOT NULL DEFAULT '{}',
  expires_at        TIMESTAMPTZ,
  archived_at       TIMESTAMPTZ,
  created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_memories_agent_id  ON memories(agent_id);
CREATE INDEX idx_memories_type      ON memories(memory_type);
CREATE INDEX idx_memories_embedding ON memories
  USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100);
