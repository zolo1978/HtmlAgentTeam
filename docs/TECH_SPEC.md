# **HARP 技术规格详细 V1.0**

> 本文档覆盖：① 完整数据库 DDL  ② WebSocket 消息协议  ③ Prolog 业务规则文件  ④ API 完整入出参规范  ⑤ 项目目录结构  ⑥ 统一错误码规范  ⑦ 部署方案  ⑧ SKILL.md 标准模板
> 与 TDD 互为补充，TDD 定架构，本文档定实现细节。

---

# **① 完整数据库 DDL**

> 执行顺序：按文档顺序依次执行，外键依赖已排好。
> 所有迁移通过 advisory lock 防并发 race（参考 adk-rust/adk-session/src/postgres.rs 模式）。

---

## **前置：启用扩展**

```sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "vector";        -- pgvector
CREATE EXTENSION IF NOT EXISTS "pg_trgm";       -- 模糊搜索
```

---

## **Migration 001 — 组织与用户**

```sql
-- 组织（多租户根节点）
CREATE TABLE organizations (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name        VARCHAR(255) NOT NULL,
  slug        VARCHAR(100) NOT NULL UNIQUE,        -- URL 友好标识，如 "acme-corp"
  plan        VARCHAR(50)  NOT NULL DEFAULT 'free', -- free / pro / enterprise
  settings    JSONB        NOT NULL DEFAULT '{}',   -- 组织级配置
  created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 用户
CREATE TABLE users (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id        UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  email         VARCHAR(255) NOT NULL,
  name          VARCHAR(255) NOT NULL,
  role          VARCHAR(50)  NOT NULL DEFAULT 'member', -- super_admin / org_admin / owner / member
  password_hash VARCHAR(255) NOT NULL,
  last_login_at TIMESTAMPTZ,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE(org_id, email)
);

CREATE INDEX idx_users_org_id ON users(org_id);
CREATE INDEX idx_users_email  ON users(email);
```

---

## **Migration 002 — Agent 核心**

```sql
-- Agent 实体
CREATE TABLE agents (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id      UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  owner_id    UUID        NOT NULL REFERENCES users(id),
  name        VARCHAR(255) NOT NULL,
  role        VARCHAR(255) NOT NULL,              -- "软件工程师" / "市场分析师" 等
  description TEXT        NOT NULL DEFAULT '',
  status      VARCHAR(50)  NOT NULL DEFAULT 'created',
  -- status: created / activated / working / reflecting / evolving / failed / archived
  settings    JSONB        NOT NULL DEFAULT '{}', -- Agent 级配置（模型偏好、语言等）
  created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_agents_org_id   ON agents(org_id);
CREATE INDEX idx_agents_status   ON agents(status);
CREATE INDEX idx_agents_owner_id ON agents(owner_id);

-- Agent 状态变更日志
CREATE TABLE agent_status_logs (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id     UUID        NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  from_status  VARCHAR(50),                        -- NULL = 初始创建
  to_status    VARCHAR(50) NOT NULL,
  reason       TEXT,                               -- 变更原因
  triggered_by VARCHAR(255),                       -- 'user:{id}' / 'system' / 'task:{id}'
  created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_agent_status_logs_agent_id   ON agent_status_logs(agent_id);
CREATE INDEX idx_agent_status_logs_created_at ON agent_status_logs(agent_id, created_at DESC);
```

---

## **Migration 003 — Task 引擎**

```sql
-- 任务
CREATE TABLE tasks (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id      UUID        NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  org_id        UUID        NOT NULL REFERENCES organizations(id),
  created_by    UUID        NOT NULL REFERENCES users(id),
  title         VARCHAR(500) NOT NULL,
  description   TEXT        NOT NULL DEFAULT '',
  priority      VARCHAR(20)  NOT NULL DEFAULT 'medium', -- low / medium / high / urgent
  status        VARCHAR(50)  NOT NULL DEFAULT 'pending',
  -- status: pending / running / completed / failed / cancelled
  result        JSONB,                              -- 任务产出结构化结果
  error_message TEXT,                               -- 失败原因
  started_at    TIMESTAMPTZ,
  completed_at  TIMESTAMPTZ,
  due_at        TIMESTAMPTZ,                        -- 截止时间（可选）
  created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tasks_agent_id   ON tasks(agent_id);
CREATE INDEX idx_tasks_status     ON tasks(status);
CREATE INDEX idx_tasks_created_at ON tasks(agent_id, created_at DESC);

-- 任务执行日志（流式写入）
CREATE TABLE task_logs (
  id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  task_id    UUID        NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  agent_id   UUID        NOT NULL REFERENCES agents(id),
  level      VARCHAR(10)  NOT NULL DEFAULT 'info', -- info / warn / error / debug
  message    TEXT        NOT NULL,
  metadata   JSONB       NOT NULL DEFAULT '{}',    -- skill_id / duration_ms 等附加信息
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_task_logs_task_id    ON task_logs(task_id, created_at ASC);
CREATE INDEX idx_task_logs_agent_id   ON task_logs(agent_id);
```

---

## **Migration 004 — Skill 引擎**

```sql
-- Skill 定义
CREATE TABLE skills (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id        UUID         NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  created_by    UUID         NOT NULL REFERENCES users(id),
  name          VARCHAR(100) NOT NULL,   -- agentskills.io 规范：小写字母+连字符，最多64字符
  description   TEXT         NOT NULL,
  skill_type    VARCHAR(50)  NOT NULL,  -- http / mcp / local / llm
  version       VARCHAR(20)  NOT NULL DEFAULT '1.0.0',
  config        JSONB        NOT NULL DEFAULT '{}',  -- endpoint/headers/timeout 等
  input_schema  JSONB        NOT NULL DEFAULT '{}',  -- JSON Schema（入参校验）
  output_schema JSONB        NOT NULL DEFAULT '{}',  -- JSON Schema（出参描述）
  permissions   JSONB        NOT NULL DEFAULT '{}',  -- {"terminal": false, "web_fetch": true}
  is_active     BOOLEAN      NOT NULL DEFAULT true,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE(org_id, name)
);

CREATE INDEX idx_skills_org_id     ON skills(org_id);
CREATE INDEX idx_skills_skill_type ON skills(skill_type);
CREATE INDEX idx_skills_is_active  ON skills(org_id, is_active);

-- Agent ↔ Skill 关联（多对多）
CREATE TABLE agent_skills (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id    UUID        NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  skill_id    UUID        NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  attached_by UUID        NOT NULL REFERENCES users(id),
  attached_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE(agent_id, skill_id)
);

CREATE INDEX idx_agent_skills_agent_id ON agent_skills(agent_id);
CREATE INDEX idx_agent_skills_skill_id ON agent_skills(skill_id);

-- Skill 执行记录
CREATE TABLE skill_execution_logs (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  skill_id      UUID        NOT NULL REFERENCES skills(id),
  agent_id      UUID        NOT NULL REFERENCES agents(id),
  task_id       UUID        REFERENCES tasks(id),
  input         JSONB       NOT NULL DEFAULT '{}',
  output        JSONB,
  error_message TEXT,
  duration_ms   INTEGER,
  status        VARCHAR(20) NOT NULL, -- success / failed / timeout
  created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_skill_exec_logs_agent_id ON skill_execution_logs(agent_id, created_at DESC);
CREATE INDEX idx_skill_exec_logs_skill_id ON skill_execution_logs(skill_id);
```

---

## **Migration 005 — Memory 引擎**

```sql
-- Memory（四层分类存储，带 pgvector 向量列）
CREATE TABLE memories (
  id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id          UUID        NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  org_id            UUID        NOT NULL REFERENCES organizations(id),
  memory_type       VARCHAR(20) NOT NULL, -- working / short / long / organization
  source            VARCHAR(50) NOT NULL,
  -- source: task_completion / reflection / manual / auto_compress / skill_execution
  content           TEXT        NOT NULL,
  compressed_content TEXT,               -- LLMLingua-2 压缩版（注入 LLM 用此版本）
  original_tokens   INTEGER,
  compressed_tokens INTEGER,
  compression_ratio NUMERIC(4,2),        -- 0.00 ~ 1.00
  embedding         vector(1536),        -- OpenAI text-embedding-3-small
  metadata          JSONB       NOT NULL DEFAULT '{}', -- task_id / skill_id / tags 等
  expires_at        TIMESTAMPTZ,         -- NULL = 永久；Working Memory 通常 24h
  archived_at       TIMESTAMPTZ,         -- 软删除时间
  created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 按 agent + 类型的常规查询索引
CREATE INDEX idx_memories_agent_type
  ON memories(agent_id, memory_type)
  WHERE archived_at IS NULL;

-- 过期清理索引
CREATE INDEX idx_memories_expires
  ON memories(expires_at)
  WHERE expires_at IS NOT NULL AND archived_at IS NULL;

-- pgvector 语义检索索引（ivfflat，100万条记录内 lists=100 合适）
CREATE INDEX idx_memories_embedding
  ON memories USING ivfflat (embedding vector_cosine_ops)
  WITH (lists = 100);
```

---

## **Migration 006 — Reflection 引擎**

```sql
CREATE TABLE reflections (
  id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id         UUID         NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  task_id          UUID         REFERENCES tasks(id),
  insights         TEXT[]       NOT NULL DEFAULT '{}',      -- 洞察列表
  next_actions     TEXT[]       NOT NULL DEFAULT '{}',      -- 下次改进行动
  skill_improvements JSONB      NOT NULL DEFAULT '{}',      -- {"skill-name": "建议..."}
  score            NUMERIC(4,2) NOT NULL DEFAULT 0,         -- 0.00 ~ 10.00
  cache_hit        BOOLEAN      NOT NULL DEFAULT false,     -- 是否命中 Semantic Cache
  prompt_tokens    INTEGER,
  completion_tokens INTEGER,
  model            VARCHAR(100),
  created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_reflections_agent_id
  ON reflections(agent_id, created_at DESC);
```

---

## **Migration 007 — Growth 引擎**

```sql
-- Agent 成长档案（每个 Agent 唯一一条）
CREATE TABLE growth_profiles (
  id               UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id         UUID         NOT NULL UNIQUE REFERENCES agents(id) ON DELETE CASCADE,
  level            INTEGER      NOT NULL DEFAULT 1,     -- 1 ~ 6
  total_score      NUMERIC(10,2) NOT NULL DEFAULT 0,
  task_score       NUMERIC(10,2) NOT NULL DEFAULT 0,    -- ×0.40 权重
  skill_score      NUMERIC(10,2) NOT NULL DEFAULT 0,    -- ×0.25 权重
  reflection_score NUMERIC(10,2) NOT NULL DEFAULT 0,   -- ×0.20 权重
  memory_score     NUMERIC(10,2) NOT NULL DEFAULT 0,    -- ×0.15 权重
  last_level_up_at TIMESTAMPTZ,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 成长历史日志
CREATE TABLE growth_logs (
  id           UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id     UUID         NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
  source       VARCHAR(50)  NOT NULL,
  -- source: task_completion / reflection / memory_write / skill_mastery / level_up_bonus
  source_id    UUID,                                    -- 来源 ID（task_id 等）
  delta        NUMERIC(6,2) NOT NULL,                  -- 本次加分（可负）
  score_after  NUMERIC(10,2) NOT NULL,                 -- 操作后总分
  level_before INTEGER      NOT NULL,
  level_after  INTEGER      NOT NULL,
  description  TEXT,
  created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_growth_logs_agent_id ON growth_logs(agent_id, created_at DESC);
```

---

## **Migration 008 — SOP 引擎**

```sql
-- SOP 定义（DAG JSON）
CREATE TABLE sops (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id      UUID         NOT NULL REFERENCES organizations(id),
  created_by  UUID         NOT NULL REFERENCES users(id),
  name        VARCHAR(255) NOT NULL,
  description TEXT,
  definition  JSONB        NOT NULL,   -- DAG 节点定义（nodes + edges + conditions）
  is_active   BOOLEAN      NOT NULL DEFAULT true,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- SOP 执行记录
CREATE TABLE sop_runs (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  sop_id        UUID        NOT NULL REFERENCES sops(id),
  agent_id      UUID        NOT NULL REFERENCES agents(id),
  status        VARCHAR(20) NOT NULL DEFAULT 'running', -- running / completed / failed / cancelled
  current_node  VARCHAR(255),                            -- 当前执行节点 ID
  state         JSONB       NOT NULL DEFAULT '{}',       -- 节点间传递的状态
  started_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  completed_at  TIMESTAMPTZ
);

CREATE INDEX idx_sop_runs_agent_id ON sop_runs(agent_id, started_at DESC);
```

---

## **Migration 009 — 事件与成本监控**

```sql
-- 事件日志（NATS 事件持久化，幂等键防重放）
CREATE TABLE event_logs (
  id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  event_type      VARCHAR(100) NOT NULL,
  agent_id        UUID         REFERENCES agents(id),
  payload         JSONB        NOT NULL,
  idempotency_key VARCHAR(255) NOT NULL UNIQUE,  -- NATS message ID
  processed_at    TIMESTAMPTZ,
  created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_event_logs_event_type ON event_logs(event_type, created_at DESC);
CREATE INDEX idx_event_logs_agent_id   ON event_logs(agent_id);

-- LLM 调用成本日志
CREATE TABLE llm_cost_logs (
  id                UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
  agent_id          UUID         REFERENCES agents(id),
  org_id            UUID         NOT NULL REFERENCES organizations(id),
  call_type         VARCHAR(50)  NOT NULL,
  -- call_type: reflection / sop_reasoning / growth_score / skill_execution / memory_embedding
  model             VARCHAR(100) NOT NULL,
  cache_layer       VARCHAR(20)  NOT NULL DEFAULT 'none',
  -- cache_layer: semantic_hit / prefix_hit / none
  prompt_tokens     INTEGER      NOT NULL DEFAULT 0,
  completion_tokens INTEGER      NOT NULL DEFAULT 0,
  cached_tokens     INTEGER      NOT NULL DEFAULT 0,  -- Prefix Cache 命中的 token 数
  cost_usd          NUMERIC(10,6) NOT NULL DEFAULT 0,
  saved_usd         NUMERIC(10,6) NOT NULL DEFAULT 0,  -- 与无缓存对比节省额
  created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_llm_cost_logs_agent_id ON llm_cost_logs(agent_id, created_at DESC);
CREATE INDEX idx_llm_cost_logs_org_id   ON llm_cost_logs(org_id, created_at DESC);

-- Semantic Response Cache（Token 优化 Layer 1）
CREATE TABLE llm_response_cache (
  id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  cache_key        VARCHAR(64)  NOT NULL,
  prompt_embedding vector(1536) NOT NULL,
  prompt_hash      VARCHAR(64)  NOT NULL,
  response         TEXT         NOT NULL,
  model            VARCHAR(100) NOT NULL,
  call_type        VARCHAR(50)  NOT NULL,
  prompt_tokens    INTEGER      NOT NULL DEFAULT 0,
  completion_tokens INTEGER     NOT NULL DEFAULT 0,
  hit_count        INTEGER      NOT NULL DEFAULT 0,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at       TIMESTAMPTZ NOT NULL
);

CREATE UNIQUE INDEX idx_llm_cache_hash      ON llm_response_cache(prompt_hash);
CREATE INDEX        idx_llm_cache_embedding ON llm_response_cache
  USING ivfflat (prompt_embedding vector_cosine_ops);
CREATE INDEX        idx_llm_cache_expires   ON llm_response_cache(expires_at);
```

---

## **Migration 010 — 定时清理任务（pg_cron）**

```sql
-- 需要 pg_cron 扩展
CREATE EXTENSION IF NOT EXISTS pg_cron;

-- 每日 03:00 UTC 清理过期 Memory
SELECT cron.schedule(
  'cleanup-expired-memories',
  '0 3 * * *',
  $$ UPDATE memories SET archived_at = NOW()
     WHERE expires_at < NOW() AND archived_at IS NULL $$
);

-- 每日 04:00 UTC 清理过期 LLM 缓存
SELECT cron.schedule(
  'cleanup-expired-llm-cache',
  '0 4 * * *',
  $$ DELETE FROM llm_response_cache WHERE expires_at < NOW() $$
);
```

---

---

# **② WebSocket 消息协议**

---

## **连接建立**

```
WS /ws/agents/{agent_id}
Header: Authorization: Bearer {access_token}
```

连接成功后服务端发送初始快照：
```json
{
  "v": 1,
  "event": "snapshot",
  "agent_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2026-06-07T10:00:00.000Z",
  "payload": {
    "status": "activated",
    "level": 2,
    "total_score": 156.5,
    "active_task": null
  }
}
```

---

## **消息基础结构**

```typescript
interface HarpWsMessage {
  v: 1;                          // 协议版本，固定为 1
  event: HarpEventType;          // 事件类型（见下表）
  agent_id: string;              // UUID
  org_id: string;                // UUID
  timestamp: string;             // ISO 8601
  payload: Record<string, unknown>;
}
```

---

## **事件类型全表**

### **Agent 状态类**

```json
// AgentStatusChanged — Agent 状态机转换
{
  "v": 1,
  "event": "AgentStatusChanged",
  "agent_id": "...",
  "org_id": "...",
  "timestamp": "2026-06-07T10:30:00.000Z",
  "payload": {
    "from_status": "activated",
    "to_status": "working",
    "triggered_by": "task:550e8400-e29b-41d4-a716-446655440001"
  }
}
```

### **Task 类**

```json
// TaskStarted
{
  "event": "TaskStarted",
  "payload": {
    "task_id": "...",
    "title": "分析 Q2 竞品报告",
    "priority": "high",
    "skill_ids": ["skill-uuid-1", "skill-uuid-2"]
  }
}

// TaskLogAppended — 流式日志（高频，每条 Skill 调用/推理步骤推一条）
{
  "event": "TaskLogAppended",
  "payload": {
    "task_id": "...",
    "log_id": "...",
    "level": "info",        // info / warn / error / debug
    "message": "调用 Skill: web-search，查询关键词：竞品定价策略",
    "metadata": {
      "skill_id": "...",
      "duration_ms": null   // 完成后更新
    }
  }
}

// TaskCompleted
{
  "event": "TaskCompleted",
  "payload": {
    "task_id": "...",
    "duration_ms": 14200,
    "reflection_triggered": true,
    "memory_written": true
  }
}

// TaskFailed
{
  "event": "TaskFailed",
  "payload": {
    "task_id": "...",
    "error_message": "Skill web-search timeout after 30s",
    "agent_status_after": "failed"
  }
}
```

### **Reflection 类**

```json
// ReflectionStarted
{
  "event": "ReflectionStarted",
  "payload": {
    "reflection_id": "...",
    "task_id": "...",
    "cache_hit": false         // true = 命中 Semantic Cache，不调用 LLM
  }
}

// ReflectionCompleted
{
  "event": "ReflectionCompleted",
  "payload": {
    "reflection_id": "...",
    "score": 7.5,
    "insights_count": 3,
    "next_actions_count": 2,
    "memory_written": true,
    "cache_hit": false,
    "tokens_used": 1240
  }
}
```

### **Growth 类**

```json
// GrowthUpdated
{
  "event": "GrowthUpdated",
  "payload": {
    "delta": 13.5,
    "score_before": 286.0,
    "score_after": 299.5,
    "level_before": 2,
    "level_after": 2,
    "leveled_up": false,
    "source": "reflection",
    "source_id": "reflection-uuid"
  }
}

// LeveledUp — 单独推送，触发前端升级动画
{
  "event": "LeveledUp",
  "payload": {
    "level_before": 2,
    "level_after": 3,
    "score": 302.0,
    "next_threshold": 700
  }
}
```

### **Memory 类**

```json
// MemoryWritten
{
  "event": "MemoryWritten",
  "payload": {
    "memory_id": "...",
    "memory_type": "long",      // working / short / long / organization
    "source": "reflection",
    "preview": "此次竞品分析任务暴露了数据收集 Skill 的不足...",
    "compressed": true
  }
}
```

### **系统类**

```json
// Ping（服务端每 30s 发送，客户端须在 10s 内回 Pong）
{ "event": "ping", "timestamp": "..." }

// Error（鉴权失败、Agent 不存在等）
{
  "event": "error",
  "payload": {
    "code": "E4041",
    "message": "Agent not found"
  }
}
```

---

## **客户端 → 服务端消息**

```json
// Pong（响应 Ping）
{ "event": "pong", "timestamp": "..." }

// Subscribe（订阅指定事件类型，默认全订阅）
{
  "event": "subscribe",
  "payload": {
    "events": ["TaskLogAppended", "AgentStatusChanged"]
  }
}
```

---

---

# **④ API 完整入出参规范**

> 所有响应 Content-Type: `application/json`
> 成功响应统一包裹：`{"data": ..., "meta": ...}`
> 错误响应统一包裹：`{"error": {"code": "E5001", "message": "...", "details": {...}}}`

---

## **通用响应结构**

```typescript
// 成功（单对象）
{ "data": T }

// 成功（列表）
{
  "data": T[],
  "meta": {
    "total": 100,
    "page": 1,
    "limit": 20,
    "has_next": true
  }
}

// 错误
{
  "error": {
    "code": "E4221",           // 业务错误码（见错误码表）
    "message": "Invalid state transition",
    "details": {               // 可选，字段级错误
      "from": "archived",
      "to": "working"
    }
  }
}
```

---

## **Auth**

### `POST /api/auth/login`

```typescript
// Request
{
  "email": "user@example.com",   // required, email format
  "password": "string"           // required, min 8 chars
}

// Response 200
{
  "data": {
    "access_token": "eyJ...",    // JWT, 15 min TTL
    "refresh_token": "eyJ...",   // JWT, 7 day TTL
    "user": {
      "id": "uuid",
      "name": "陈伟峰",
      "email": "user@example.com",
      "role": "org_admin",
      "org_id": "uuid"
    }
  }
}

// Error 401: E4010 Invalid credentials
// Error 429: E4290 Rate limit exceeded (5次/min)
```

### `POST /api/auth/refresh`

```typescript
// Request
{ "refresh_token": "eyJ..." }

// Response 200
{
  "data": {
    "access_token": "eyJ...",    // 新 access_token
    "refresh_token": "eyJ..."   // 滚动刷新，旧 token 立即失效
  }
}
```

---

## **Agent**

### `POST /api/agents`

```typescript
// Request
{
  "name": "竞品分析 Agent",       // required, 1-255 chars
  "role": "市场分析师",           // required, 1-255 chars
  "description": "专注于竞品...", // optional, max 2000 chars
  "org_id": "uuid"               // required（从 JWT 取，可省略）
}

// Response 201
{
  "data": {
    "id": "uuid",
    "name": "竞品分析 Agent",
    "role": "市场分析师",
    "description": "...",
    "status": "created",
    "owner_id": "uuid",
    "org_id": "uuid",
    "created_at": "2026-06-07T10:00:00Z",
    "updated_at": "2026-06-07T10:00:00Z"
  }
}

// Error 422: E4221 Validation failed (name too long etc.)
```

### `GET /api/agents`

```typescript
// Query params
{
  "status"?: "created|activated|working|reflecting|evolving|failed|archived",
  "page"?: number,      // default 1
  "limit"?: number,     // default 20, max 100
  "search"?: string     // 模糊匹配 name/role
}

// Response 200 — 列表 + 简化 growth 信息
{
  "data": [
    {
      "id": "uuid",
      "name": "竞品分析 Agent",
      "role": "市场分析师",
      "status": "working",
      "level": 2,
      "total_score": 156.5,
      "skill_count": 3,
      "last_task_at": "2026-06-07T09:30:00Z",
      "created_at": "..."
    }
  ],
  "meta": { "total": 42, "page": 1, "limit": 20, "has_next": true }
}
```

### `GET /api/agents/{id}`

```typescript
// Response 200 — 完整详情
{
  "data": {
    "id": "uuid",
    "name": "竞品分析 Agent",
    "role": "市场分析师",
    "description": "...",
    "status": "working",
    "owner_id": "uuid",
    "org_id": "uuid",
    "growth": {
      "level": 2,
      "total_score": 156.5,
      "next_threshold": 300,
      "breakdown": {
        "task_score": 80.0,
        "skill_score": 30.5,
        "reflection_score": 30.0,
        "memory_score": 16.0
      }
    },
    "skills": [
      { "id": "uuid", "name": "web-search", "skill_type": "http", "is_active": true }
    ],
    "recent_tasks": [
      { "id": "uuid", "title": "分析 Q2 竞品", "status": "completed", "completed_at": "..." }
    ],
    "memory_summary": {
      "working_count": 3,
      "short_count": 12,
      "long_count": 45,
      "org_count": 8
    },
    "created_at": "...",
    "updated_at": "..."
  }
}

// Error 404: E4041 Agent not found
// Error 403: E4031 No permission to access this agent
```

### `PATCH /api/agents/{id}`

```typescript
// Request（所有字段可选，至少传一个）
{
  "name"?: "新名称",
  "role"?: "新角色",
  "description"?: "新描述"
  // 注意：status 不可通过 PATCH 直接修改，须用专用接口
}

// Response 200 — 更新后的 Agent（结构同 GET /{id} 但无 skills/tasks）
```

### `POST /api/agents/{id}/activate`

```typescript
// Request: 无 body
// Response 200
{
  "data": {
    "id": "uuid",
    "status": "activated",
    "status_changed_at": "2026-06-07T10:05:00Z"
  }
}
// Error 422: E4222 Invalid state transition (e.g., archived → activated)
```

---

## **Task**

### `POST /api/tasks`

```typescript
// Request
{
  "agent_id": "uuid",           // required
  "title": "分析 Q2 竞品报告", // required, 1-500 chars
  "description": "重点关注...", // optional
  "priority": "high",          // optional, default "medium"
  "due_at": "2026-06-08T18:00:00Z" // optional
}

// Response 201
{
  "data": {
    "id": "uuid",
    "agent_id": "uuid",
    "title": "...",
    "status": "pending",
    "priority": "high",
    "created_at": "..."
  }
}
```

### `POST /api/tasks/{id}/run`

```typescript
// Request: 无 body（或可选覆盖参数）
// Response 202 — 异步触发，立即返回
{
  "data": {
    "task_id": "uuid",
    "agent_id": "uuid",
    "status": "running",
    "started_at": "2026-06-07T10:10:00Z",
    "ws_channel": "/ws/agents/{agent_id}"  // 通过此 WS 通道接收实时日志
  }
}
// Error 409: E4091 Task already running
// Error 422: E4223 Agent not in activated status
```

### `GET /api/tasks/{id}/logs` （SSE）

```
Content-Type: text/event-stream

data: {"log_id":"uuid","level":"info","message":"开始执行任务","created_at":"..."}
data: {"log_id":"uuid","level":"info","message":"调用 Skill: web-search","created_at":"..."}
data: {"log_id":"uuid","level":"warn","message":"Skill 返回空结果，重试...","created_at":"..."}
data: [DONE]
```

---

## **Skill**

### `POST /api/skills`

```typescript
// Request
{
  "name": "web-search",          // required, agentskills.io 规范命名
  "description": "搜索互联网",   // required
  "skill_type": "http",          // required: http / mcp / local / llm
  "version": "1.0.0",            // optional
  "config": {                    // skill_type=http 时
    "endpoint": "https://api.search.com/v1/search",
    "method": "POST",
    "headers": { "Authorization": "Bearer {env:SEARCH_API_KEY}" },
    "timeout_ms": 30000
  },
  "input_schema": {              // JSON Schema
    "type": "object",
    "required": ["query"],
    "properties": {
      "query": { "type": "string", "description": "搜索关键词" },
      "limit": { "type": "integer", "default": 10 }
    }
  },
  "output_schema": {
    "type": "object",
    "properties": {
      "results": { "type": "array", "items": { "type": "string" } }
    }
  }
}

// Response 201
{
  "data": {
    "id": "uuid",
    "name": "web-search",
    "skill_type": "http",
    "version": "1.0.0",
    "is_active": true,
    "created_at": "..."
  }
}
```

### `POST /api/skills/{id}/execute`

```typescript
// Request（透传给 Skill 的入参，须符合 input_schema）
{
  "query": "Claude 竞品定价策略",
  "limit": 5
}

// Response 200
{
  "data": {
    "output": { "results": ["..."] },   // 符合 output_schema
    "duration_ms": 1240,
    "status": "success"
  }
}
// Error 422: E4224 Input schema validation failed
// Error 504: E5041 Skill timeout
```

---

## **Memory**

### `POST /api/memories`

```typescript
// Request
{
  "agent_id": "uuid",            // required
  "content": "此次任务发现...",   // required, max 10000 chars（原始内容）
  "memory_type": "short",        // required: working / short / long / organization
  "source": "manual",            // required: task_completion / reflection / manual / skill_execution
  "metadata": {                  // optional
    "task_id": "uuid",
    "tags": ["竞品", "定价"]
  },
  "expires_at": "2026-06-14T00:00:00Z" // optional，不填 = 永久
}

// Response 201（同步返回，embedding 异步生成）
{
  "data": {
    "id": "uuid",
    "memory_type": "short",
    "status": "indexing",        // embedding 生成中
    "created_at": "..."
  }
}
```

### `POST /api/memories/search`

```typescript
// Request
{
  "agent_id": "uuid",            // required
  "query": "竞品定价分析",        // required
  "top_k": 5,                   // optional, default 5, max 20
  "min_score": 0.75,            // optional, default 0.75（余弦相似度阈值）
  "memory_types": ["short", "long"] // optional，不填 = 全部类型
}

// Response 200
{
  "data": [
    {
      "id": "uuid",
      "memory_type": "long",
      "content": "竞品分析发现...",          // 原始内容
      "compressed_content": "竞品分析...",   // 压缩版
      "score": 0.912,                         // 余弦相似度
      "source": "reflection",
      "created_at": "..."
    }
  ],
  "meta": {
    "query_embedding_ms": 45,
    "search_ms": 12,
    "total_found": 3
  }
}
```

---

## **Reflection**

### `GET /api/agents/{id}/reflections`

```typescript
// Query params: page, limit, task_id(optional)

// Response 200
{
  "data": [
    {
      "id": "uuid",
      "task_id": "uuid",
      "score": 7.5,
      "insights": ["主要洞察1", "主要洞察2"],
      "next_actions": ["下次改进1"],
      "cache_hit": false,
      "created_at": "..."
    }
  ],
  "meta": { "total": 15, ... }
}
```

---

## **Growth**

### `GET /api/agents/{id}/growth`

```typescript
// Response 200
{
  "data": {
    "agent_id": "uuid",
    "level": 2,
    "total_score": 156.5,
    "next_threshold": 300,
    "progress_pct": 52.2,         // (156.5 - 100) / (300 - 100) × 100
    "breakdown": {
      "task_score":       80.0,   // 完成任务贡献
      "skill_score":      30.5,   // Skill 熟练贡献
      "reflection_score": 30.0,   // 复盘贡献
      "memory_score":     16.0    // 记忆积累贡献
    },
    "last_level_up_at": "2026-06-01T14:00:00Z",
    "updated_at": "..."
  }
}
```

### `GET /api/agents/{id}/growth/logs`

```typescript
// Query params: page, limit, source(optional)

// Response 200
{
  "data": [
    {
      "id": "uuid",
      "source": "reflection",
      "source_id": "reflection-uuid",
      "delta": 13.5,
      "score_after": 156.5,
      "level_before": 2,
      "level_after": 2,
      "description": "Reflection 高分（7.5/10），奖励成长值",
      "created_at": "..."
    }
  ],
  "meta": { "total": 28, ... }
}
```

---

## **错误码对照表**

| **HTTP** | **错误码** | **含义** | **触发场景** |
|---|---|---|---|
| 400 | E4000 | Bad Request | 通用参数错误 |
| 401 | E4010 | Unauthorized | Token 无效或过期 |
| 401 | E4011 | Token Expired | access_token 过期，用 refresh |
| 403 | E4031 | Forbidden | 无权访问该资源 |
| 404 | E4041 | Not Found | Agent/Task/Skill 不存在 |
| 409 | E4091 | Conflict | 任务已在运行中 |
| 422 | E4221 | Validation Failed | 入参字段校验失败 |
| 422 | E4222 | Invalid State Transition | Agent 状态机非法转换 |
| 422 | E4223 | Agent Not Ready | Agent 未激活，不能执行任务 |
| 422 | E4224 | Skill Schema Mismatch | 入参不符合 Skill input_schema |
| 429 | E4290 | Rate Limited | 超过 API 速率限制 |
| 500 | E5000 | Internal Error | 服务器内部错误 |
| 500 | E5001 | Database Error | DB 操作失败 |
| 500 | E5002 | LLM Error | LLM 调用失败 |
| 500 | E5003 | Memory Write Failed | Memory 写入或 embedding 失败 |
| 504 | E5041 | Skill Timeout | Skill 执行超时（> 30s） |

---

---

# **⑤ 项目目录结构**

---

## **Monorepo 根目录**

```
harp/                             ← 项目根
├── backend/                      ← Rust 后端
├── frontend/                     ← Next.js 15 前端
├── infra/                        ← 基础设施配置
├── .github/
│   └── workflows/
│       ├── ci.yml                ← PR 检查（lint + test + build）
│       └── deploy.yml            ← main 分支自动部署
├── .env.example                  ← 环境变量模板
└── README.md
```

---

## **Backend 目录（Rust）**

```
backend/
├── Cargo.toml                    ← workspace 依赖声明
├── Cargo.lock
├── src/
│   ├── main.rs                   ← 启动入口（AppState 组装 + Router 启动）
│   ├── lib.rs                    ← 公开 pub use（测试用）
│   ├── config.rs                 ← AppConfig（从 env 加载，startup 校验）
│   ├── state.rs                  ← AppState { pg_pool, redis, nats, rig_client }
│   ├── error.rs                  ← AppError enum + IntoResponse（HTTP 映射）
│   ├── router.rs                 ← Axum Router 路由注册总表
│   │
│   ├── middleware/
│   │   ├── mod.rs
│   │   ├── auth.rs               ← JWT 验证 Layer（提取 Claims 注入 Extensions）
│   │   └── rate_limit.rs         ← tower-governor 配置（per-IP + per-user）
│   │
│   ├── db/
│   │   ├── mod.rs
│   │   └── migrations.rs         ← advisory lock migration runner（from adk-rust）
│   │
│   ├── agent/                    ← Agent 核心模块
│   │   ├── mod.rs
│   │   ├── model.rs              ← Agent struct, AgentStatus enum
│   │   ├── state_machine.rs      ← transition() + 合法转换表 + 日志写入
│   │   ├── repository.rs         ← DB CRUD（sqlx queries）
│   │   ├── handlers.rs           ← Axum HTTP handlers（POST/GET/PATCH）
│   │   ├── ws_handler.rs         ← WebSocket handler + 消息推送
│   │   └── goal_store.rs         ← 跨 Session 目标注入（Redis TTL）
│   │
│   ├── task/
│   │   ├── mod.rs
│   │   ├── model.rs              ← Task struct, TaskStatus enum
│   │   ├── executor.rs           ← autoagents ReAct loop 封装
│   │   ├── repository.rs
│   │   └── handlers.rs
│   │
│   ├── skill/
│   │   ├── mod.rs
│   │   ├── model.rs              ← Skill struct, SkillType enum
│   │   ├── parser.rs             ← parse_skill_md（adapted from garudust）
│   │   ├── loader.rs             ← load_skills_from_dir（adapted from garudust）
│   │   ├── permissions.rs        ← SkillPermissions deny-wins（from garudust）
│   │   ├── executor.rs           ← autoagents #[tool] dispatch + 执行记录
│   │   ├── repository.rs
│   │   └── handlers.rs
│   │
│   ├── memory/
│   │   ├── mod.rs
│   │   ├── model.rs              ← Memory struct, MemoryType enum
│   │   ├── validation.rs         ← validate_memory_content（from garudust）
│   │   ├── write_pipeline.rs     ← swiftide indexing pipeline（PgVector 写入）
│   │   ├── query_pipeline.rs     ← swiftide query pipeline（语义检索）
│   │   ├── injection.rs          ← TokenWindowMemory 2000 token 预算（from rig-memory）
│   │   ├── compressor.rs         ← LLMLingua-2 HTTP bridge（写入时压缩）
│   │   ├── auto_summarizer.rs    ← 定时摘要（Short>30 → Long，Long>100 → 聚类）
│   │   ├── repository.rs
│   │   └── handlers.rs
│   │
│   ├── reflection/
│   │   ├── mod.rs
│   │   ├── model.rs              ← ReflectionOutput struct（rig ExtractorAgent 用）
│   │   ├── extractor.rs          ← rig ExtractorAgent 配置（adapted from rig-core）
│   │   ├── prompt.rs             ← 静态前缀（cache_control）+ 动态后缀构建
│   │   ├── cache.rs              ← Semantic Response Cache Layer 1（pgvector 查询）
│   │   ├── consumer.rs           ← NATS TaskCompleted 消费者
│   │   ├── repository.rs
│   │   └── handlers.rs
│   │
│   ├── growth/
│   │   ├── mod.rs
│   │   ├── model.rs              ← GrowthProfile, GrowthLog struct
│   │   ├── calculator.rs         ← GrowthScore 公式 + 等级判定（L1-L6 阈值）
│   │   ├── consumer.rs           ← NATS ReflectionCompleted + TaskCompleted 消费者
│   │   ├── repository.rs
│   │   └── handlers.rs
│   │
│   ├── sop/
│   │   ├── mod.rs
│   │   ├── model.rs              ← Sop struct, SopNode, SopRun
│   │   ├── dag_runner.rs         ← DAG 节点执行器（含条件跳转 + 异常恢复）
│   │   ├── repository.rs
│   │   └── handlers.rs
│   │
│   ├── prolog/
│   │   ├── mod.rs
│   │   ├── scryer_runtime.rs     ← （直接从 PrologAgentTeam 复制）
│   │   ├── rule_store.rs         ← （直接从 PrologAgentTeam 复制）
│   │   ├── git_manager.rs        ← （直接从 PrologAgentTeam 复制）
│   │   └── rules/               ← 业务规则 .pl 文件
│   │       ├── task_assignment.pl
│   │       ├── level_up.pl
│   │       ├── sop_conditions.pl
│   │       ├── memory_validation.pl
│   │       └── growth_triggers.pl
│   │
│   ├── event/
│   │   ├── mod.rs
│   │   ├── types.rs              ← HarpEvent enum（所有事件类型）
│   │   ├── publisher.rs          ← NATS publish（序列化 + 幂等键）
│   │   └── subscriber.rs         ← NATS consumer group（at-least-once）
│   │
│   ├── llm/
│   │   ├── mod.rs
│   │   ├── client.rs             ← rig-core wrapper（统一 LLM 入口）
│   │   └── cost_tracker.rs       ← LlmCostLog 异步写入
│   │
│   └── auth/
│       ├── mod.rs
│       ├── model.rs              ← Claims struct, TokenPair
│       └── handlers.rs           ← login / refresh / logout
│
└── tests/                        ← 集成测试（独立于 src）
    ├── common/
    │   └── setup.rs              ← 测试 DB 初始化 + 清理
    ├── test_agent_state_machine.rs
    ├── test_memory_integration.rs
    ├── test_skill_execution.rs
    ├── test_reflection_integration.rs
    ├── test_growth_engine.rs
    └── test_token_optimization.rs
```

---

## **Frontend 目录（Next.js 15）**

```
frontend/
├── src/
│   ├── app/                     ← Next.js App Router
│   │   ├── layout.tsx            ← 全局 Layout（字体 + Provider）
│   │   ├── page.tsx              ← redirect → /agents
│   │   ├── (auth)/
│   │   │   └── login/page.tsx
│   │   ├── agents/
│   │   │   ├── page.tsx          ← Agent 列表页
│   │   │   └── [id]/
│   │   │       └── page.tsx      ← Agent 详情页（HTML Agent 主页）
│   │   └── orgs/
│   │       └── settings/page.tsx ← 组织设置
│   │
│   ├── components/
│   │   ├── ui/                  ← shadcn/ui 原子组件（不改动）
│   │   ├── agent/
│   │   │   ├── AgentCard.tsx         ← 列表卡片
│   │   │   ├── AgentStatusBadge.tsx  ← 状态彩色圆点
│   │   │   ├── AgentDetailPanel.tsx  ← 详情页左侧信息栏
│   │   │   └── CreateAgentModal.tsx  ← 创建对话框
│   │   ├── task/
│   │   │   ├── TaskTimeline.tsx      ← 垂直时间线（SSE 实时追加）
│   │   │   ├── TaskStatusPill.tsx    ← 状态胶囊
│   │   │   └── CreateTaskModal.tsx
│   │   ├── memory/
│   │   │   ├── MemoryPanel.tsx       ← 四层 Memory 分组展示
│   │   │   ├── MemoryChip.tsx        ← 单条 Memory 卡片
│   │   │   └── MemorySearch.tsx      ← 语义搜索框
│   │   ├── skill/
│   │   │   ├── SkillPanel.tsx        ← Agent 已挂载 Skill 列表
│   │   │   └── SkillTag.tsx          ← Skill 名称 + 类型图标
│   │   ├── growth/
│   │   │   ├── GrowthBar.tsx         ← 成长值进度条
│   │   │   ├── GrowthLevelBadge.tsx  ← L1-L6 等级徽章
│   │   │   └── LevelUpModal.tsx      ← 升级庆祝 Modal（1200ms 动画）
│   │   └── reflection/
│   │       └── ReflectionCard.tsx    ← 折叠展示复盘内容
│   │
│   ├── store/                   ← Zustand 状态管理
│   │   ├── agentStore.ts         ← Agent 列表 + 详情
│   │   ├── taskStore.ts          ← 任务列表 + 实时日志
│   │   ├── wsStore.ts            ← WebSocket 连接 + 事件分发
│   │   └── authStore.ts          ← 用户 + Token
│   │
│   ├── hooks/
│   │   ├── useAgent.ts           ← Agent 数据获取 + 操作
│   │   ├── useWebSocket.ts       ← WS 连接管理 + 重连
│   │   └── useGrowth.ts          ← 成长数据 + 等级计算
│   │
│   ├── lib/
│   │   ├── api.ts               ← fetch 封装（自动注 Authorization header）
│   │   └── ws.ts                ← WebSocket 客户端类（重连 + 心跳）
│   │
│   └── styles/
│       └── tokens.css           ← Design Token CSS 变量（全部颜色/圆角/间距）
│
├── tests/
│   ├── components/              ← Vitest 组件快照测试
│   └── e2e/                     ← Playwright E2E
│       └── agent-lifecycle.spec.ts
│
├── tailwind.config.ts
├── next.config.ts
└── tsconfig.json
```

---

## **Infra 目录**

```
infra/
├── docker-compose.yml           ← 本地开发（PG + pgvector + Redis + NATS）
├── docker-compose.staging.yml   ← Staging 环境覆盖配置
└── .env.example                 ← 所有必填环境变量模板
```

---

## `.env.example` 完整清单

```bash
# === 数据库 ===
DATABASE_URL=postgres://harp:harp_dev_2026@localhost:5432/harp
REDIS_URL=redis://localhost:6379
NATS_URL=nats://localhost:4222

# === LLM ===
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
# 可选：其他 provider（rig-core 支持 20+）
# DEEPSEEK_API_KEY=
# GROQ_API_KEY=

# === 鉴权 ===
JWT_SECRET=your-256-bit-secret-here      # 生产：openssl rand -hex 32
JWT_ACCESS_TTL_MINUTES=15
JWT_REFRESH_TTL_DAYS=7

# === Token 优化 ===
SEMANTIC_CACHE_THRESHOLD=0.92            # Semantic Cache 相似度阈值
MEMORY_INJECTION_BUDGET_TOKENS=2000      # Memory 注入硬上限
REFLECTION_CACHE_TTL_HOURS=24
COMPRESSOR_SERVICE_URL=http://localhost:8001  # LLMLingua-2 压缩服务

# === 成本监控 ===
COST_ALERT_THRESHOLD_USD_PER_AGENT=0.10  # 超过此值触发告警

# === 应用 ===
RUST_LOG=info
PORT=8080
FRONTEND_URL=http://localhost:3000       # CORS 白名单
```

---

# **③ Prolog 业务规则文件**

> 运行引擎：scryer-prolog 0.9（纯 Rust 实现）。
> 文件位置：`harp/backend/src/prolog/rules/`
> Rust 侧通过 `scryer_prolog::Machine` 加载并查询；结果以 JSON 序列化传回。
> 风格：声明式优先，终止条件在前，不使用 cut(!) 除非注释说明原因。

---

## **task_assignment.pl — 任务分配规则**

```prolog
%% task_assignment.pl
%% 决定哪些 Agent 可以接受某个 Task。
%% 供 Rust TaskService::find_eligible_agents(task_id) 调用。
%%
%% 外部事实（由 Rust 在调用前 assertz 注入）：
%%   task_type(TaskId, Type)          % Type: atom, 如 'coding'
%%   task_priority(TaskId, Priority)  % Priority: high / normal / low
%%   task_sla_hours(TaskId, Hours)    % 截止时间余量（整数小时）
%%   agent_skill(AgentId, Skill)      % Agent 拥有的技能类型
%%   agent_status(AgentId, Status)    % created/activated/working/reflecting/...
%%   agent_active_tasks(AgentId, N)   % 当前持有任务数
%%   agent_level(AgentId, Level)      % L1..L6

:- module(task_assignment, [can_assign/2, best_agent/3]).

%% can_assign(+TaskId, +AgentId)
%% 成功条件：AgentId 可以被分配 TaskId。
can_assign(TaskId, AgentId) :-
    task_type(TaskId, Type),
    agent_skill(AgentId, Type),           % 具备所需技能
    agent_status(AgentId, activated),     % 处于可工作状态
    \+ agent_overloaded(AgentId).         % 未过载

%% agent_overloaded(+AgentId)
%% Agent 被认为过载：已有任务 >= 3。
agent_overloaded(AgentId) :-
    agent_active_tasks(AgentId, N),
    N >= 3.

%% high_priority_constraint(+TaskId, +AgentId)
%% 高优先级任务要求 Agent 级别 >= L3。
high_priority_constraint(TaskId, AgentId) :-
    task_priority(TaskId, high),
    agent_level(AgentId, Level),
    level_value(Level, V),
    V >= 3.

high_priority_constraint(TaskId, _AgentId) :-
    \+ task_priority(TaskId, high).       % 非高优，无约束

%% level_value(+LevelAtom, -IntValue)
level_value('L1', 1).
level_value('L2', 2).
level_value('L3', 3).
level_value('L4', 4).
level_value('L5', 5).
level_value('L6', 6).

%% eligible_agent(+TaskId, -AgentId)
%% 同时满足基础 + 优先级约束的 Agent。
eligible_agent(TaskId, AgentId) :-
    can_assign(TaskId, AgentId),
    high_priority_constraint(TaskId, AgentId).

%% best_agent(+TaskId, +CandidateList, -BestAgentId)
%% 从候选列表中选出任务数最少的 Agent（负载均衡）。
best_agent(TaskId, Candidates, Best) :-
    include(eligible_agent(TaskId), Candidates, Eligible),
    Eligible \= [],                       % 至少一个合格 Agent
    find_min_load(Eligible, Best).

find_min_load([Agent], Agent) :- !.
find_min_load([A|Rest], Best) :-
    find_min_load(Rest, BestOfRest),
    agent_active_tasks(A, NA),
    agent_active_tasks(BestOfRest, NB),
    (NA =< NB -> Best = A ; Best = BestOfRest).
```

---

## **level_up.pl — 升级判断规则**

```prolog
%% level_up.pl
%% 判断 Agent 是否满足升级条件。
%% 供 Rust GrowthService::check_level_up(agent_id) 调用。
%%
%% 外部事实（Rust 注入）：
%%   agent_growth_score(AgentId, Score)    % 当前 GrowthScore（浮点）
%%   agent_level(AgentId, CurrentLevel)    % 当前等级 atom
%%   agent_completed_tasks(AgentId, N)     % 累计完成任务数
%%   agent_reflection_count(AgentId, N)    % 累计反思次数
%%   agent_skill_count(AgentId, N)         % 掌握技能数

:- module(level_up, [should_level_up/2, next_level/2, level_requirements_met/2]).

%% level_threshold(+Level, -MinScore)
%% 达到该等级所需的最低 GrowthScore。
level_threshold('L1', 0).
level_threshold('L2', 100).
level_threshold('L3', 300).
level_threshold('L4', 700).
level_threshold('L5', 1500).
level_threshold('L6', 3000).

%% next_level(+Current, -Next)
next_level('L1', 'L2').
next_level('L2', 'L3').
next_level('L3', 'L4').
next_level('L4', 'L5').
next_level('L5', 'L6').
% L6 是最高等级，无 next_level/2 子句，查询自动失败。

%% min_tasks_for_level(+Level, -MinTasks)
%% 晋升到该等级需要的最低累计任务数。
min_tasks_for_level('L2', 5).
min_tasks_for_level('L3', 20).
min_tasks_for_level('L4', 60).
min_tasks_for_level('L5', 150).
min_tasks_for_level('L6', 400).

%% min_reflections_for_level(+Level, -MinReflections)
min_reflections_for_level('L2', 3).
min_reflections_for_level('L3', 10).
min_reflections_for_level('L4', 30).
min_reflections_for_level('L5', 80).
min_reflections_for_level('L6', 200).

%% score_qualifies(+AgentId, +TargetLevel)
score_qualifies(AgentId, TargetLevel) :-
    agent_growth_score(AgentId, Score),
    level_threshold(TargetLevel, MinScore),
    Score >= MinScore.

%% tasks_qualifies(+AgentId, +TargetLevel)
tasks_qualifies(AgentId, TargetLevel) :-
    agent_completed_tasks(AgentId, N),
    min_tasks_for_level(TargetLevel, Min),
    N >= Min.

%% reflections_qualify(+AgentId, +TargetLevel)
reflections_qualify(AgentId, TargetLevel) :-
    agent_reflection_count(AgentId, N),
    min_reflections_for_level(TargetLevel, Min),
    N >= Min.

%% level_requirements_met(+AgentId, +TargetLevel)
%% 全部条件满足才可升级。
level_requirements_met(AgentId, TargetLevel) :-
    score_qualifies(AgentId, TargetLevel),
    tasks_qualifies(AgentId, TargetLevel),
    reflections_qualify(AgentId, TargetLevel).

%% should_level_up(+AgentId, -NewLevel)
%% 若满足条件返回 NewLevel，否则查询失败。
should_level_up(AgentId, NewLevel) :-
    agent_level(AgentId, CurrentLevel),
    next_level(CurrentLevel, NewLevel),
    level_requirements_met(AgentId, NewLevel).
```

---

## **sop_conditions.pl — SOP 节点前置条件**

```prolog
%% sop_conditions.pl
%% 判断 SOP（标准作业流程）中某个节点的前置条件是否满足。
%% 供 Rust SopRunner::can_start_node(sop_id, node_id) 调用。
%%
%% 外部事实（Rust 注入）：
%%   node_dependency(SopId, NodeId, DepNodeId) % NodeId 依赖 DepNodeId 完成
%%   node_status(SopId, NodeId, Status)         % pending/running/completed/failed/skipped
%%   node_type(SopId, NodeId, Type)             % sequential/parallel/conditional
%%   condition_var(SopId, VarName, Value)       % 运行时条件变量

:- module(sop_conditions, [can_start/2, is_blocked/2, all_deps_done/2]).

%% node_done(+SopId, +NodeId)
%% 节点处于已完成或跳过状态。
node_done(SopId, NodeId) :-
    node_status(SopId, NodeId, completed).
node_done(SopId, NodeId) :-
    node_status(SopId, NodeId, skipped).

%% all_deps_done(+SopId, +NodeId)
%% 该节点的所有依赖节点都已完成。
all_deps_done(SopId, NodeId) :-
    findall(Dep, node_dependency(SopId, NodeId, Dep), Deps),
    maplist(node_done(SopId), Deps).

%% no_failed_deps(+SopId, +NodeId)
%% 无失败的前置节点。
no_failed_deps(SopId, NodeId) :-
    \+ (node_dependency(SopId, NodeId, Dep),
        node_status(SopId, Dep, failed)).

%% can_start(+SopId, +NodeId)
%% 节点可以开始执行。
can_start(SopId, NodeId) :-
    node_status(SopId, NodeId, pending),   % 必须处于等待状态
    all_deps_done(SopId, NodeId),          % 所有依赖已完成
    no_failed_deps(SopId, NodeId).         % 没有失败的上游

%% is_blocked(+SopId, +NodeId)
%% 节点被上游阻塞（有未完成的依赖）。
is_blocked(SopId, NodeId) :-
    node_dependency(SopId, NodeId, Dep),
    \+ node_done(SopId, Dep).

%% ready_nodes(+SopId, -ReadyList)
%% 找出 SOP 中所有当前可启动的节点。
ready_nodes(SopId, ReadyList) :-
    findall(N, can_start(SopId, N), ReadyList).

%% sop_complete(+SopId)
%% SOP 已完成：不存在 pending 或 running 节点。
sop_complete(SopId) :-
    \+ node_status(SopId, _, pending),
    \+ node_status(SopId, _, running).
```

---

## **memory_validation.pl — 记忆内容校验规则**

```prolog
%% memory_validation.pl
%% 校验准备写入的 Memory 内容是否合法。
%% 供 Rust MemoryService::validate_before_write(memory) 调用。
%%
%% 外部事实（Rust 注入）：
%%   proposed_memory(AgentId, Type, Content, TokenCount)
%%     Type: working/short_term/long_term/organization
%%   org_memory_count(OrgId, Count)     % 组织记忆当前数量
%%   agent_memory_count(AgentId, Count) % Agent 记忆当前数量

:- module(memory_validation, [memory_valid/1, validation_errors/2]).

%% 各类型 Token 上限
max_tokens(working,      500).
max_tokens(short_term,   200).
max_tokens(long_term,    400).
max_tokens(organization, 300).

%% 各类型单 Agent 数量上限
max_count_per_agent(working,       5).
max_count_per_agent(short_term,   30).
max_count_per_agent(long_term,   100).
max_count_per_agent(organization, 50).

%% valid_type(+Type)
valid_type(working).
valid_type(short_term).
valid_type(long_term).
valid_type(organization).

%% check_type(+Type, -Error)
check_type(Type, ok) :- valid_type(Type), !.
check_type(Type, invalid_type(Type)).

%% check_tokens(+Type, +TokenCount, -Error)
check_tokens(Type, Count, ok) :-
    max_tokens(Type, Max),
    Count =< Max, !.
check_tokens(Type, Count, token_limit_exceeded(Type, Count)).

%% check_content_nonempty(+Content, -Error)
check_content_nonempty(Content, ok) :-
    Content \= '', Content \= [], !.
check_content_nonempty(_, empty_content).

%% check_count_limit(+AgentId, +Type, -Error)
check_count_limit(AgentId, Type, ok) :-
    agent_memory_count(AgentId, Count),
    max_count_per_agent(Type, Max),
    Count < Max, !.
check_count_limit(AgentId, Type, count_limit_exceeded(AgentId, Type)).

%% collect_errors(+AgentId, +Type, +Content, +Tokens, -Errors)
collect_errors(AgentId, Type, Content, Tokens, Errors) :-
    check_type(Type, E1),
    check_tokens(Type, Tokens, E2),
    check_content_nonempty(Content, E3),
    check_count_limit(AgentId, Type, E4),
    exclude(==(ok), [E1, E2, E3, E4], Errors).

%% validation_errors(+AgentId, -Errors)
%% 收集所有校验错误，供 Rust 侧格式化输出。
validation_errors(AgentId, Errors) :-
    proposed_memory(AgentId, Type, Content, Tokens),
    collect_errors(AgentId, Type, Content, Tokens, Errors).

%% memory_valid(+AgentId)
%% 无校验错误则通过。
memory_valid(AgentId) :-
    validation_errors(AgentId, []).
```

---

## **growth_triggers.pl — 成长事件触发规则**

```prolog
%% growth_triggers.pl
%% 判断某个事件是否触发成长分加成。
%% 供 Rust GrowthService::apply_event(agent_id, event) 调用。
%%
%% 外部事实（Rust 注入）：
%%   agent_level(AgentId, Level)
%%   event(AgentId, EventType, Metadata)
%%     EventType: task_completed / task_failed / reflection_done /
%%                skill_used / memory_recalled / goal_achieved
%%   task_quality(TaskId, Score)      % 0.0..1.0，任务质量评分
%%   skill_first_use(AgentId, SkillId) % 是否首次使用该技能

:- module(growth_triggers, [event_score/3, bonus_multiplier/3, total_score/3]).

%% base_score(+EventType, -Score)
%% 事件基础分。
base_score(task_completed,  40).
base_score(reflection_done, 20).
base_score(skill_used,      25).
base_score(goal_achieved,   60).
base_score(memory_recalled,  5).
base_score(task_failed,      0).  % 失败不得分，但不扣分

%% quality_bonus(+EventType, +Metadata, -Bonus)
%% 高质量任务完成给予额外奖励。
quality_bonus(task_completed, Metadata, Bonus) :-
    get_dict(task_id, Metadata, TaskId),
    task_quality(TaskId, Q),
    Q >= 0.9,
    !,
    Bonus = 20.
quality_bonus(task_completed, Metadata, Bonus) :-
    get_dict(task_id, Metadata, TaskId),
    task_quality(TaskId, Q),
    Q >= 0.7,
    !,
    Bonus = 10.
quality_bonus(_, _, 0).

%% first_use_bonus(+AgentId, +EventType, +Metadata, -Bonus)
%% 首次使用新技能额外奖励。
first_use_bonus(AgentId, skill_used, Metadata, 15) :-
    get_dict(skill_id, Metadata, SkillId),
    skill_first_use(AgentId, SkillId), !.
first_use_bonus(_, _, _, 0).

%% level_multiplier(+Level, -Multiplier)
%% 高等级 Agent 降低收益（避免马太效应）。
level_multiplier('L1', 1.2).
level_multiplier('L2', 1.1).
level_multiplier('L3', 1.0).
level_multiplier('L4', 0.9).
level_multiplier('L5', 0.85).
level_multiplier('L6', 0.8).

%% bonus_multiplier(+AgentId, +EventType, -Multiplier)
bonus_multiplier(AgentId, _EventType, Multiplier) :-
    agent_level(AgentId, Level),
    level_multiplier(Level, Multiplier).

%% event_score(+AgentId, +EventType, +Metadata, -TotalScore)
%% 计算本次事件的实际成长分。
event_score(AgentId, EventType, Metadata, TotalScore) :-
    base_score(EventType, Base),
    quality_bonus(EventType, Metadata, QBonus),
    first_use_bonus(AgentId, EventType, Metadata, FBonus),
    bonus_multiplier(AgentId, EventType, Multiplier),
    Raw is (Base + QBonus + FBonus) * Multiplier,
    TotalScore is round(Raw).

%% total_score(+AgentId, +EventList, -Sum)
%% 批量计算多个事件的总分。
total_score(_, [], 0).
total_score(AgentId, [event(Type, Meta)|Rest], Sum) :-
    event_score(AgentId, Type, Meta, S),
    total_score(AgentId, Rest, RestSum),
    Sum is S + RestSum.
```

---

## **Rust 侧调用示例**

```rust
// backend/src/prolog/engine.rs
use scryer_prolog::machine::Machine;

pub struct PrologEngine {
    machine: Machine,
}

impl PrologEngine {
    pub fn new() -> Self {
        let mut machine = Machine::new_lib();
        // 加载规则文件
        machine.consult_module_string(
            "task_assignment",
            include_str!("rules/task_assignment.pl"),
        );
        machine.consult_module_string(
            "level_up",
            include_str!("rules/level_up.pl"),
        );
        // ... 其余规则文件
        Self { machine }
    }

    /// 查询：TaskId 可分配给哪些 Agent
    pub async fn eligible_agents(
        &mut self,
        task_id: Uuid,
        task_type: &str,
        task_priority: &str,
        candidates: &[(Uuid, &str, i32, i32)], // (agent_id, status, tasks, level)
    ) -> Vec<Uuid> {
        // 注入事实
        let facts = format!(
            "task_type({tid}, {typ}). task_priority({tid}, {pri}).",
            tid = quoted(task_id),
            typ = task_type,
            pri = task_priority,
        );
        self.machine.assert_facts(&facts);
        for (aid, status, n_tasks, level) in candidates {
            self.machine.assert_fact(&format!(
                "agent_status({aid}, {status}). agent_active_tasks({aid}, {n}). agent_level({aid}, {lvl}).",
                aid = quoted(*aid), status = status, n = n_tasks, lvl = level_atom(*level),
            ));
        }
        // 查询
        let results = self.machine.run_query(
            "task_assignment:eligible_agent(TaskId, AgentId)."
        );
        // 解析并返回
        results.into_iter()
            .filter_map(|sol| sol.get("AgentId").and_then(|v| v.as_uuid()))
            .collect()
    }
}

fn level_atom(level: i32) -> &'static str {
    match level {
        1 => "'L1'", 2 => "'L2'", 3 => "'L3'",
        4 => "'L4'", 5 => "'L5'", _ => "'L6'",
    }
}
```

---

# **⑥ 统一错误码规范**

> 原则：所有 API 响应 JSON 结构一致；错误码机器可读、前端可 i18n。

---

## **HTTP 响应信封**

```rust
// backend/src/error.rs

/// 请求元信息（每个响应都携带）
#[derive(Serialize)]
pub struct ApiMeta {
    pub request_id: Uuid,       // Uuid::now_v7()，用于链路追踪
    pub timestamp: i64,         // Unix 毫秒时间戳
    pub page: Option<PageMeta>, // 分页信息（非分页接口为 None）
}

#[derive(Serialize)]
pub struct PageMeta {
    pub next_cursor: Option<String>, // base64(created_at_ms:id)
    pub has_more: bool,
    pub total: Option<i64>,          // 非必须，昂贵查询可省略
}

/// 成功响应
#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,   // 始终 true
    pub data: T,
    pub meta: ApiMeta,   // 始终携带 request_id + timestamp
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data,
            meta: ApiMeta {
                request_id: Uuid::now_v7(),
                timestamp: Utc::now().timestamp_millis(),
                page: None,
            },
        }
    }

    pub fn paged(data: T, next_cursor: Option<String>, has_more: bool) -> Self {
        Self {
            success: true,
            data,
            meta: ApiMeta {
                request_id: Uuid::now_v7(),
                timestamp: Utc::now().timestamp_millis(),
                page: Some(PageMeta { next_cursor, has_more, total: None }),
            },
        }
    }
}

/// 错误响应
#[derive(Serialize)]
pub struct ApiError {
    pub success: bool,   // 始终 false
    pub error: ErrorBody,
    pub meta: ApiMeta,   // 始终携带 request_id 以便客户端上报
}

#[derive(Serialize)]
pub struct ErrorBody {
    pub code: &'static str,               // 机器可读码，如 "E4010"
    pub message: String,                  // 人类可读描述（英文）
    pub details: Option<serde_json::Value>, // 字段级错误、调试信息等
}
```

---

## **AppError 枚举 → HTTP 映射**

```rust
// backend/src/error.rs (续)
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // ── 4xx ──────────────────────────────────────────
    #[error("not found: {resource}")]
    NotFound { resource: String },

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("token expired")]
    TokenExpired,

    #[error("token invalid")]
    TokenInvalid,

    #[error("unauthorized: {action}")]
    Unauthorized { action: String },

    #[error("forbidden")]
    Forbidden,

    #[error("validation failed")]
    ValidationFailed { fields: Vec<FieldError> },

    #[error("invalid state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("rate limit exceeded")]
    RateLimitExceeded,

    #[error("conflict: {resource}")]
    Conflict { resource: String },

    // ── 5xx ──────────────────────────────────────────
    #[error("database error")]
    Database(#[from] sqlx::Error),

    #[error("llm provider error: {provider}")]
    LlmProvider { provider: String, message: String },

    #[error("prolog engine error")]
    PrologEngine(String),

    #[error("event bus error")]
    EventBus(String),

    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::NotFound { .. }            => "E4040",
            AppError::InvalidCredentials         => "E4010",
            AppError::TokenExpired               => "E4011",
            AppError::TokenInvalid               => "E4012",
            AppError::Unauthorized { .. }        => "E4013",
            AppError::Forbidden                  => "E4030",
            AppError::ValidationFailed { .. }    => "E4220",
            AppError::InvalidStateTransition {..}=> "E4221",
            AppError::RateLimitExceeded          => "E4290",
            AppError::Conflict { .. }            => "E4090",
            AppError::Database(_)                => "E5000",
            AppError::LlmProvider { .. }         => "E5010",
            AppError::PrologEngine(_)            => "E5020",
            AppError::EventBus(_)                => "E5030",
            AppError::Internal(_)                => "E5040",
        }
    }

    pub fn http_status(&self) -> StatusCode {
        match self {
            AppError::NotFound { .. }            => StatusCode::NOT_FOUND,
            AppError::InvalidCredentials         => StatusCode::UNAUTHORIZED,
            AppError::TokenExpired               => StatusCode::UNAUTHORIZED,
            AppError::TokenInvalid               => StatusCode::UNAUTHORIZED,
            AppError::Unauthorized { .. }        => StatusCode::UNAUTHORIZED,
            AppError::Forbidden                  => StatusCode::FORBIDDEN,
            AppError::ValidationFailed { .. }    => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::InvalidStateTransition {..}=> StatusCode::UNPROCESSABLE_ENTITY,
            AppError::RateLimitExceeded          => StatusCode::TOO_MANY_REQUESTS,
            AppError::Conflict { .. }            => StatusCode::CONFLICT,
            _                                    => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.http_status();
        let details = match &self {
            AppError::ValidationFailed { fields } =>
                Some(serde_json::to_value(fields).unwrap_or_default()),
            AppError::InvalidStateTransition { from, to } =>
                Some(serde_json::json!({ "from": from, "to": to })),
            AppError::LlmProvider { provider, message } =>
                Some(serde_json::json!({ "provider": provider, "message": message })),
            _ => None,
        };
        let body = ApiError {
            success: false,
            error: ErrorBody {
                code: self.code(),
                message: self.to_string(),
                details,
            },
            meta: ApiMeta {
                request_id: Uuid::now_v7(),
                timestamp: Utc::now().timestamp_millis(),
                page: None,
            },
        };
        (status, Json(body)).into_response()
    }
}
```

---

## **错误码速查表**

| Code  | HTTP | 含义                      | 触发场景                            |
|-------|------|---------------------------|-------------------------------------|
| E4010 | 401  | invalid_credentials       | 用户名或密码错误                    |
| E4011 | 401  | token_expired             | JWT Access Token 过期               |
| E4012 | 401  | token_invalid             | JWT 签名不合法                      |
| E4013 | 401  | unauthorized              | 缺少必要权限                        |
| E4030 | 403  | forbidden                 | 越权访问（组织隔离）                |
| E4040 | 404  | not_found                 | 资源不存在                          |
| E4090 | 409  | conflict                  | Email 重复/slug 冲突等              |
| E4220 | 422  | validation_failed         | 请求字段校验失败，含 fields 列表    |
| E4221 | 422  | invalid_state_transition  | Agent/Task 非法状态流转             |
| E4290 | 429  | rate_limit_exceeded       | 超出 API 频率限制                   |
| E5000 | 500  | database_error            | 数据库连接/查询失败                 |
| E5010 | 500  | llm_provider_error        | LLM API 超时或错误                  |
| E5020 | 500  | prolog_engine_error       | Prolog 推理引擎故障                 |
| E5030 | 500  | event_bus_error           | NATS 发布/消费失败                  |
| E5040 | 500  | internal_error            | 未分类内部错误                      |

---

## **JWT Claims 规范**

```rust
// backend/src/auth/jwt.rs（改自 RustAgentTeam jwt-auth.rs 模板）

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Access Token Claims（15 min TTL）
#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: Uuid,           // user_id
    pub org_id: Uuid,        // 组织隔离关键字段
    pub role: String,        // super_admin / org_admin / owner / member
    pub exp: usize,          // Unix 时间戳（秒）
    pub iat: usize,          // 签发时间
    pub jti: Uuid,           // JWT ID（用于 revoke）
}

/// Refresh Token Claims（7 day TTL）
#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub sub: Uuid,           // user_id
    pub org_id: Uuid,
    pub exp: usize,
    pub iat: usize,
    pub jti: Uuid,           // 滚动刷新：旧 jti 失效后 Redis 删除
    pub family: Uuid,        // Refresh Token Family（检测 Token Reuse）
}

/// 从 Authorization: Bearer <token> 提取 AuthUser（Axum extractor）
pub struct AuthUser {
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub role: String,
    pub jti: Uuid,
}

#[axum::async_trait]
impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, state: &Arc<AppState>) -> Result<Self, Self::Rejection> {
        let bearer = parts.headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(AppError::TokenInvalid)?;
        let tok = decode::<AccessClaims>(
            bearer,
            &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
            &Validation::default(),
        ).map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
            _ => AppError::TokenInvalid,
        })?;
        Ok(AuthUser {
            user_id: tok.claims.sub,
            org_id: tok.claims.org_id,
            role: tok.claims.role,
            jti: tok.claims.jti,
        })
    }
}

/// 生成 Token 对
pub fn generate_token_pair(
    user_id: Uuid,
    org_id: Uuid,
    role: &str,
    secret: &str,
) -> Result<(String, String), AppError> {
    let now = Utc::now().timestamp() as usize;
    let family = Uuid::now_v7();

    let access = encode(
        &Header::default(),
        &AccessClaims {
            sub: user_id, org_id, role: role.to_string(),
            exp: now + 15 * 60,  // 15 min
            iat: now, jti: Uuid::now_v7(),
        },
        &EncodingKey::from_secret(secret.as_bytes()),
    ).map_err(|e| AppError::Internal(e.into()))?;

    let refresh = encode(
        &Header::default(),
        &RefreshClaims {
            sub: user_id, org_id,
            exp: now + 7 * 24 * 3600,  // 7 days
            iat: now, jti: Uuid::now_v7(), family,
        },
        &EncodingKey::from_secret(secret.as_bytes()),
    ).map_err(|e| AppError::Internal(e.into()))?;

    Ok((access, refresh))
}
```

---

## **前端错误处理约定**

```typescript
// frontend/src/lib/api-client.ts
interface HarpApiError {
  success: false;
  error: {
    code: string;       // "E4010"
    message: string;    // 英文技术描述
    details?: unknown;  // 结构化补充
  };
}

// 前端按 code 显示本地化消息
const ERROR_I18N: Record<string, string> = {
  E4010: '用户名或密码错误',
  E4011: '登录已过期，请重新登录',
  E4012: '无效的登录凭证',
  E4220: '请检查表单填写',
  E4221: '当前状态不允许此操作',
  E4290: '操作过于频繁，请稍后再试',
  E5000: '服务暂时不可用，请稍后重试',
};
```

---

# **⑦ 部署方案**

> 决策原则：MVP 先用最低成本跑通完整链路，再按需升级。

---

## **三阶段部署路线图**

```
Phase 1 (MVP · 0-3月)      Phase 2 (Beta · 3-9月)      Phase 3 (GA · 9月+)
┌─────────────────────┐    ┌─────────────────────────┐   ┌──────────────────────┐
│  Railway / Render   │───▶│  Fly.io (多区域)         │──▶│  K8s (自建/GKE/EKS)  │
│  单实例，一键部署    │    │  水平扩展，全球 CDN       │   │  多租户隔离，SLA 99.9%│
└─────────────────────┘    └─────────────────────────┘   └──────────────────────┘
```

---

## **Phase 1 — Railway（MVP 推荐）**

选择理由：`Cargo.toml` 即部署，PostgreSQL + Redis 一键开通，月费约 $20-40，无 DevOps 负担。

```yaml
# railway.toml（放在 monorepo 根）
[build]
builder = "dockerfile"
dockerfilePath = "harp/backend/Dockerfile"

[deploy]
startCommand = "./harp-backend"
healthcheckPath = "/api/v1/health"
healthcheckTimeout = 30

[[services]]
name = "harp-backend"
source = "harp/backend"

[[services]]
name = "harp-frontend"
source = "harp/frontend"
```

```dockerfile
# harp/backend/Dockerfile
FROM rust:1.79-slim AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/harp-backend /usr/local/bin/harp-backend
EXPOSE 8080
CMD ["harp-backend"]
```

Railway 服务清单（MVP）：

| 服务                | 规格        | 月费约   |
|---------------------|-------------|---------|
| harp-backend (Rust) | 512MB RAM   | $5      |
| harp-frontend (Node)| 512MB RAM   | $5      |
| PostgreSQL + pgvector | 1GB       | $5      |
| Redis               | 256MB       | $3      |
| NATS (单节点)       | 256MB       | $3      |
| **合计**            |             | **~$21**|

> NATS JetStream 在 Railway 可用自定义 Docker 镜像 `nats:2.10-alpine`。

---

## **Phase 2 — Fly.io（Beta 扩展）**

```toml
# fly.toml
app = "harp-backend"
primary_region = "hkg"   # 香港，延迟优先

[http_service]
  internal_port = 8080
  force_https = true
  auto_stop_machines = false
  min_machines_running = 2    # 至少2实例，高可用

[[vm]]
  cpu_kind = "shared"
  cpus = 2
  memory_mb = 1024

[mounts]
  source = "harp_data"
  destination = "/var/data"
```

Phase 2 增加：
- `fly scale count 3` 水平扩展
- Fly Postgres（托管，自动 pgvector 扩展）
- Cloudflare CDN + R2 对象存储（SKILL.md 文件）
- Upstash Redis（全球边缘缓存）

---

## **Phase 3 — K8s（GA 生产）**

```yaml
# k8s/backend-deployment.yaml（骨架）
apiVersion: apps/v1
kind: Deployment
metadata:
  name: harp-backend
  namespace: harp-prod
spec:
  replicas: 3
  selector:
    matchLabels: { app: harp-backend }
  template:
    metadata:
      labels: { app: harp-backend }
    spec:
      containers:
      - name: backend
        image: ghcr.io/your-org/harp-backend:latest
        ports: [{ containerPort: 8080 }]
        envFrom:
        - secretRef: { name: harp-secrets }
        resources:
          requests: { cpu: "250m", memory: "256Mi" }
          limits:   { cpu: "1000m", memory: "512Mi" }
        livenessProbe:
          httpGet: { path: /api/v1/health, port: 8080 }
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet: { path: /api/v1/health, port: 8080 }
          initialDelaySeconds: 5
```

---

## **CI/CD — GitHub Actions**

```yaml
# .github/workflows/deploy.yml
name: Deploy

on:
  push:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: pgvector/pgvector:pg16
        env:
          POSTGRES_PASSWORD: harp_test
          POSTGRES_DB: harp_test
        ports: ["5432:5432"]
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - name: Run tests
      run: cargo test --workspace
      env:
        DATABASE_URL: postgres://postgres:harp_test@localhost:5432/harp_test

  deploy-backend:
    needs: test
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - name: Deploy to Railway
      uses: bervProject/railway-deploy@main
      with:
        railway_token: ${{ secrets.RAILWAY_TOKEN }}
        service: harp-backend

  deploy-frontend:
    needs: test
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - name: Deploy to Vercel
      uses: amondnet/vercel-action@v25
      with:
        vercel-token: ${{ secrets.VERCEL_TOKEN }}
        vercel-org-id: ${{ secrets.ORG_ID }}
        vercel-project-id: ${{ secrets.PROJECT_ID }}
        working-directory: harp/frontend
```

---

## **环境矩阵**

| 环境    | 后端         | 前端    | DB              | 说明           |
|---------|-------------|---------|-----------------|----------------|
| dev     | `cargo run` | `next dev` | Docker Compose | 本地开发       |
| staging | Railway     | Vercel Preview | Railway PG  | PR 自动部署    |
| prod    | Railway/Fly | Vercel  | Railway PG / Fly PG | main 分支推送 |

---

# **⑧ SKILL.md 标准模板**

> 格式基准：参考 promptAgentTeam/SKILL.md。
> 位置：每个 Agent 的家园目录 `harp/agents/<agent-id>/SKILL.md`。
> 解析器：`garudust-agent/crates/garudust-tools/src/toolsets/skills.rs::parse_skill_md`。

---

```markdown
---
name: <技能唯一标识，如 code-review>
description: >
  一句话描述技能能力与触发条件。触发词: [关键词1, 关键词2]
version: "1.0.0"
author: <agent-id 或 user-id>
permissions:
  read: [memory, tasks, skills]       # 可读资源
  write: [memory]                     # 可写资源
  execute: [llm, tools]               # 可执行操作
disable-model-invocation: false       # true 则只用规则，不调 LLM
user-invocable: true                  # 是否可被人类用户直接触发
---

# <技能名称>（人类可读标题）

## 定位

一段话描述：这个技能解决什么问题，适用场景，不适用场景。

## 触发条件

- 触发词 / 关键词
- 上下文条件（如：任务类型为 X）
- 前置状态要求（如：Agent 处于 activated）

## 能力边界

| 能做 | 不能做 |
|------|-------|
| 具体能力A | 超出边界B |
| 具体能力C | 超出边界D |

## 执行流程

1. **Step 1**：描述步骤，输入/输出
2. **Step 2**：描述步骤，输入/输出
3. **Step 3**：描述步骤，输入/输出

每步完成后对照验收条件检查，未通过则重做。

## 验收条件

- [ ] 条件1（可量化，如：返回 JSON 包含 result 字段）
- [ ] 条件2
- [ ] 条件3

## 示例

### 输入

\`\`\`
用户或系统给技能的典型输入示例
\`\`\`

### 输出

\`\`\`json
{
  "result": "...",
  "confidence": 0.9,
  "notes": "..."
}
\`\`\`

## 错误处理

| 错误情形 | 处理方式 |
|---------|---------|
| 输入为空 | 返回 E4220 ValidationFailed |
| LLM 超时 | 重试一次，失败返回 E5010 |
| 权限不足 | 返回 E4030 Forbidden |

## 依赖

- 技能依赖的其他技能 ID（如有）
- 依赖的外部工具或服务

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | YYYY-MM-DD | 初始版本 |
```

---

## **内置技能示例：code-review**

```markdown
---
name: code-review
description: >
  审查 Rust 代码，检查安全性、性能、可读性。
  触发词: [review, 审查, code review, 代码审查]
version: "1.0.0"
author: system
permissions:
  read: [memory, tasks, skills]
  write: [memory]
  execute: [llm]
disable-model-invocation: false
user-invocable: true
---

# Code Review 技能

## 定位

对提交的 Rust 代码进行多维度审查，输出结构化报告。
不适用：需要运行代码或访问外部服务的审查场景。

## 触发条件

- 任务类型包含 "review" 或 "审查"
- 用户提交了代码块且要求评估

## 执行流程

1. **理解上下文**：读取相关记忆，了解 Agent 技术偏好
2. **静态分析**：检查所有权/借用/生命周期错误
3. **模式识别**：unwrap/clone 滥用、不合理 unsafe
4. **输出报告**：CRITICAL / HIGH / MEDIUM / LOW 分级

## 验收条件

- [ ] 报告包含 severity 分级
- [ ] 每个问题有具体行号
- [ ] 提供改进建议代码
```

---

## **SKILL.md 字段规范**

| 字段                    | 类型    | 必填 | 说明                                    |
|------------------------|---------|------|-----------------------------------------|
| `name`                 | string  | ✅   | 唯一标识，kebab-case，不含特殊字符        |
| `description`          | string  | ✅   | 含触发词的一句话描述，≤300 字符           |
| `version`              | semver  | ✅   | "major.minor.patch"                     |
| `author`               | string  | ✅   | agent-id 或 user-id                     |
| `permissions.read`     | array   | ✅   | 可读资源列表                             |
| `permissions.write`    | array   | ✅   | 可写资源列表                             |
| `permissions.execute`  | array   | ✅   | 可执行操作列表                           |
| `disable-model-invocation` | bool | ✅  | false=可调 LLM；true=仅规则              |
| `user-invocable`       | bool    | ✅   | 是否可被人类用户直接触发                  |

> **解析器行为**（来自 garudust parse_skill_md）：
> - YAML frontmatter（`---` 包裹）为结构化元数据
> - frontmatter 之后的 Markdown 为人类可读技能说明
> - `name` 字段校验：`^[a-z0-9][a-z0-9\-]{0,63}$`
> - `permissions` 缺字段时默认为空数组（不报错）
