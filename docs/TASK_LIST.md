# **HARP 实施任务总清单 V1.0**

> 本文档是 HARP 项目的**唯一执行入口**。
> TDD 是架构规格，Backlog 是 Sprint 计划，本文档是**逐条可勾选的行动清单**。
> 每个任务只有两个状态：`- [ ]` 未完成 / `- [x]` 完成。

---

## **状态速查**

| **维度** | **任务总数** | **完成** | **当前阻塞** |
|---|---|---|---|
| A. 代码萃取 | 18 | 0 | 等待 Sprint 0 启动 |
| B. 技术架构确认 | 14 | 12 | API Server 鉴权方案待定 |
| C. API 接口规范 | 36 | 0 | 等待 B 完成 |
| D. 设计规范 | 22 | 0 | 等待 Sprint 0 |
| E. 测试范例 | 24 | 0 | 等待 Sprint 0 |
| F. 开发实施规划 | 98 | 0 | Sprint 0 待启动 |

---

---

# **A. 可萃取代码清单**

> 来源：`~/Projects/harp/research/` 五个仓库
> 操作类型：🟢 直接复制 / 🟡 适配修改 / 🔵 参考模式

---

## **A1. Memory 层萃取**

- [ ] 🟢 **复制 `validate_memory_content`**
  - 来源：`garudust-agent/crates/garudust-core/src/memory.rs` L1-50
  - 目标：`harp/backend/src/memory/validation.rs`
  - 内容：XML 注入防御 + 500 字符限制 + `strip_angle_brackets`
  - 改动：字符上限改为 2000，新增 HARP 分层校验（WorkingMemory 无上限）

- [ ] 🟢 **复制 `MemoryCategory` 枚举**
  - 来源：`garudust-agent/crates/garudust-core/src/memory.rs` MemoryCategory
  - 目标：`harp/backend/src/memory/types.rs`
  - 改动：枚举名改为 `MemoryType`，成员改为 `Working/Short/Long/Organization`

- [ ] 🟡 **适配 swiftide pgvector 写入 pipeline**
  - 来源：`swiftide/examples/index_md_into_pgvector.rs`
  - 目标：`harp/backend/src/memory/write_pipeline.rs`
  - 改动：替换 FastEmbed → OpenAI text-embedding-3-small；加 agent_id / memory_type / expires_at metadata；pipeline 末端加压缩步骤

- [ ] 🟡 **适配 swiftide query pipeline（Memory 检索）**
  - 来源：`swiftide/examples/query_pipeline.rs`
  - 目标：`harp/backend/src/memory/query_pipeline.rs`
  - 改动：替换 Qdrant → PgVector；加 agent_id 过滤；加 min_score 阈值门控；禁用 GenerateSubquestions（HARP 直接用任务描述作 query）

- [ ] 🔵 **参考 garudust GoalStore 模式实现跨 Session 目标注入**
  - 来源：`garudust-agent/crates/garudust-memory/src/goal_store.rs`
  - 目标：`harp/backend/src/agent/goal_store.rs`
  - 模式：session_key → hash → 文件持久化，改为 Redis TTL 存储

---

## **A2. Skill 层萃取**

- [ ] 🟢 **复制 `parse_skill_md` + `Skill` struct**
  - 来源：`garudust-agent/crates/garudust-tools/src/toolsets/skills.rs` L1-100
  - 目标：`harp/backend/src/skill/parser.rs`
  - 改动：frontmatter 新增字段 `skill_type`（HTTP/MCP/Local/LLM）、`input_schema`（JSON Schema）、`output_schema`

- [ ] 🟢 **复制 `load_skills_from_dir` + `build_skills_index`**
  - 来源：`garudust-agent/crates/garudust-tools/src/toolsets/skills.rs` L100-160
  - 目标：`harp/backend/src/skill/loader.rs`
  - 改动：目录从 `~/.garudust/skills/` 改为 HARP DB 查询；加 org_id 隔离

- [ ] 🟡 **适配 `WriteSkill` 工具（Agent 运行时自建 Skill）**
  - 来源：`garudust-agent/crates/garudust-tools/src/toolsets/skills.rs` WriteSkill
  - 目标：`harp/backend/src/skill/write_skill_tool.rs`
  - 改动：写入目标从文件系统改为 HARP DB；关联 agent_id

- [ ] 🟢 **复制 `SkillPermissions::merge`（deny-wins 权限合并）**
  - 来源：`garudust-agent/crates/garudust-core/src/memory.rs` SkillPermissions
  - 目标：`harp/backend/src/skill/permissions.rs`
  - 改动：接入 HARP RBAC 角色体系，Owner 可覆盖 deny

- [ ] 🟢 **复制 `sanitize_skill_name`（agentskills.io 兼容名称校验）**
  - 来源：`garudust-agent/crates/garudust-tools/src/toolsets/skills.rs` L160-185
  - 目标：`harp/backend/src/skill/parser.rs` 同文件
  - 改动：无需改动，直接用

---

## **A3. Session / 持久化层萃取**

- [ ] 🟡 **适配 ADK-Rust PostgresSessionService 三层 state 设计**
  - 来源：`adk-rust/adk-session/src/postgres.rs` L1-80
  - 目标：`harp/backend/src/agent/session_store.rs`
  - 改动：三层 state 从 `app/user/session` 改为 `org/agent/task`；advisory lock 模式完整保留；JSONB 列完整保留

- [ ] 🟡 **适配 ADK-Rust PG Migration Advisory Lock**
  - 来源：`adk-rust/adk-session/src/postgres.rs` ADVISORY_LOCK_KEY + PG_SESSION_MIGRATIONS
  - 目标：`harp/backend/src/db/migrations.rs`
  - 改动：把 HARP 所有表 DDL 按此模式做 migration；防并发 race

---

## **A5. RustAgentTeam 萃取（新增，来自 research/RustAgentTeam）**

> 优先级：所有 A5 任务在 Sprint 0 前完成，它们是 HARP 后端骨架的直接原材料。

- [ ] 🟢 **复制 `axum-endpoint.rs` 模板**
  - 来源：`RustAgentTeam/skills/rust-backend/templates/axum-endpoint.rs`
  - 目标：`harp/backend/src/api/template.rs`（仅做参考，实际按 36 个端点各自实现）
  - 模式：`State<Arc<AppState>>` + `Json(req): Json<Req>` + `req.validate()` + `ApiResponse::success(dto)`

- [ ] 🟡 **适配 `jwt-auth.rs` 模板（Claims 加 org_id）**
  - 来源：`RustAgentTeam/skills/rust-backend/templates/jwt-auth.rs`
  - 目标：`harp/backend/src/auth/jwt.rs`
  - 改动：`Claims` 新增 `org_id: Uuid`；`AuthUser` 新增 `org_id`；`exp` 改为 15 分钟；新增 `refresh_token` 生成函数（7 天）

- [ ] 🟡 **适配 `paged-list.rs` 模板（Cursor 分页）**
  - 来源：`RustAgentTeam/skills/rust-backend/templates/paged-list.rs`
  - 目标：`harp/backend/src/api/pagination.rs`
  - 改动：cursor = `base64(created_at_ms:id)` 完整保留；`limit` 默认 20，上限 100（HARP 规格）

- [ ] 🟢 **复制 `ApiResponse<T>` 完整实现（含 request_id + timestamp）**
  - 来源：`RustAgentTeam/skills/rust-backend/references/error-codes.md` ApiResponse struct
  - 目标：`harp/backend/src/api/response.rs`
  - 改动：`meta.request_id` 改用 `Uuid::now_v7()`；`meta.timestamp` 保留毫秒级 Unix 时间戳
  - 注意：⑥ 规格中的 ApiResponse 缺少 `meta` 字段，以此模板为准

- [ ] 🟢 **复制 Axum oneshot 测试模式**
  - 来源：`RustAgentTeam/skills/rust-backend/templates/test-helpers.rs` Pattern 2
  - 目标：`harp/backend/src/api/tests/helpers.rs`
  - 改动：`test_state()` 改为注入 test PostgreSQL（sqlx::PgPool，test DB）

- [ ] 🔵 **参考 BAD/GOOD 对比：一 Command 一事、内部模型→DTO、分页**
  - 来源：`RustAgentTeam/skills/rust-backend/references/bad-good-comparisons.md`
  - 用途：Sprint 0 Code Review 时的 Checklist 参考
  - 行动：把 3 个 BAD/GOOD 对比加入 Sprint 0 自审清单

---

## **A4. LLM / Token 优化层萃取**

- [ ] 🟡 **适配 rig TokenWindowMemory 实现 Memory 注入 2000 token 预算**
  - 来源：`rig/crates/rig-memory/examples/agent_with_memory_policies.rs`
  - 目标：`harp/backend/src/memory/injection.rs`
  - 改动：预算从 `256` 改为 `2000`；token 计数器改用 tiktoken-rs

- [ ] 🟡 **适配 rig ExtractorAgent 实现 Reflection 结构化输出**
  - 来源：`rig/crates/rig-core/src/extractor.rs`
  - 目标：`harp/backend/src/reflection/extractor.rs`
  - 改动：定义 `ReflectionOutput` struct（insights/next_actions/skill_improvements/score）；派生 `schemars::JsonSchema`

- [ ] 🟡 **适配 autoagents ReAct executor 接入 HARP Skill 执行**
  - 来源：`AutoAgents/crates/autoagents-core/` + `examples/basic/`
  - 目标：`harp/backend/src/skill/executor.rs`
  - 改动：用 `#[tool]` 宏注册 HARP Skill；`AgentBuilder` 注入 Memory

---

---

# **B. 最终技术架构确认清单**

> 每条都是已确认决策，打勾表示团队对齐完成。

---

## **B1. 后端运行时**

- [x] Rust + Tokio 异步运行时（不用 Python/Node）
- [x] Axum 0.7 作为 HTTP Server
- [x] async-nats 0.35 作为 Event Bus（NATS JetStream，持久化）
- [x] scryer-prolog 嵌入主进程（不独立部署 SWI-Prolog）
- [x] sqlx 0.7 直接操作 PostgreSQL（不用 ORM）

## **B2. LLM / Agent 执行层**

- [x] rig-core 0.38 作为 LLM 客户端（20+ provider）
- [x] rig-memory 0.38 的 `TokenWindowMemory` 实现注入预算
- [x] rig-postgres 0.38 的 `PostgresVectorStore` 实现向量检索
- [x] autoagents 0.32 ReAct executor 执行 Skill
- [x] swiftide 0.38 作为 Memory RAG pipeline（写入 + 检索）

## **B3. 数据层**

- [x] PostgreSQL 16 + pgvector（vector(1536)，OpenAI text-embedding-3-small）
- [x] Redis 7 作为 Agent 状态热缓存 + Session
- [ ] **待确认**：Semantic Response Cache 用 pgvector 还是独立 Redis（建议 pgvector，零新增基础设施）

## **B4. 前端**

- [x] Next.js 15 + Zustand + shadcn/ui + Tailwind + Framer Motion
- [ ] **待确认**：WebSocket 用 Axum 原生 WS 还是 Socket.io 桥接（建议 Axum 原生）

---

---

# **C. API 接口完整规范清单**

> 每条任务 = 完成该接口的完整规范（入参/出参/错误码/示例）

---

## **C1. Auth（鉴权）**

- [ ] `POST /api/auth/login` — 规范入参（email+password）、出参（access_token+refresh_token）、错误（401/429）
- [ ] `POST /api/auth/refresh` — 规范 refresh_token 滚动更新逻辑
- [ ] `POST /api/auth/logout` — 规范 token 失效逻辑

## **C2. Agent（核心）**

- [ ] `POST /api/agents` — 入参：`{name, role, description, org_id}`，出参：`Agent`，状态初始为 `Created`
- [ ] `GET /api/agents` — 支持分页（`page/limit`）+ 过滤（`status/org_id`）
- [ ] `GET /api/agents/{id}` — 出参：`Agent` + `growth_summary` + `recent_tasks[5]`
- [ ] `PATCH /api/agents/{id}` — 只允许改 `name/description/role`，status 不可直接 PATCH
- [ ] `POST /api/agents/{id}/activate` — 触发 `Created→Activated` 状态机
- [ ] `POST /api/agents/{id}/archive` — 触发 `*/Activated→Archived` 状态机
- [ ] `GET /api/agents/{id}/status-logs` — 出参：状态变更历史列表
- [ ] `WS /ws/agents/{id}` — WebSocket 推送：`{event, agent_id, status, timestamp}`

## **C3. Task（任务）**

- [ ] `POST /api/tasks` — 入参：`{agent_id, title, description, priority, due_at}`
- [ ] `GET /api/tasks` — 支持 `agent_id/status/date_range` 过滤
- [ ] `GET /api/tasks/{id}` — 出参：Task + `execution_logs[]`
- [ ] `POST /api/tasks/{id}/run` — 触发 Agent 执行，返回 `task_run_id`
- [ ] `POST /api/tasks/{id}/cancel` — 中止执行中任务
- [ ] `GET /api/tasks/{id}/logs` — SSE 流式返回实时执行日志

## **C4. Skill（技能）**

- [ ] `POST /api/skills` — 入参：`{name, description, skill_type, config, org_id}`
- [ ] `GET /api/skills` — 支持 `skill_type/org_id` 过滤
- [ ] `POST /api/skills/{id}/execute` — 直接调用 Skill（测试用途），入参透传给 Skill
- [ ] `POST /api/agents/{agent_id}/skills/{skill_id}` — 为 Agent 挂载 Skill
- [ ] `DELETE /api/agents/{agent_id}/skills/{skill_id}` — 移除 Agent 的 Skill

## **C5. Memory（记忆）**

- [ ] `POST /api/memories` — 入参：`{agent_id, content, memory_type, source}`，自动触发 embedding
- [ ] `POST /api/memories/search` — 入参：`{agent_id, query, top_k, min_score, memory_types[]}`，语义检索
- [ ] `GET /api/agents/{id}/memories` — 按分层列举（`memory_type` 过滤）
- [ ] `DELETE /api/memories/{id}` — 软删除（标记 archived_at）

## **C6. Reflection（复盘）**

- [ ] `GET /api/agents/{id}/reflections` — 分页列表
- [ ] `GET /api/reflections/{id}` — 出参：完整复盘内容（insights/next_actions/score）
- [ ] `POST /api/reflections` — 手动触发复盘（传 task_id）

## **C7. Growth（成长）**

- [ ] `GET /api/agents/{id}/growth` — 出参：`{level, score, breakdown, next_threshold}`
- [ ] `GET /api/agents/{id}/growth/logs` — 成长历史（含每次加分的来源和分值）

## **C8. LLM Cost（成本监控）**

- [ ] `GET /api/agents/{id}/cost` — 出参：`{today_usd, month_usd, cache_hit_rate, tokens_per_reflection}`
- [ ] `GET /api/orgs/{id}/cost` — 组织级成本汇总

---

---

# **D. 设计规范完整清单**

---

## **D1. Design Token（必须在 Sprint 0 完成）**

- [ ] 建 `harp/frontend/src/styles/tokens.css` — 写入所有 CSS 变量（Agent 状态色 7 个 + 等级色 6 个 + 语义色）
- [ ] 建 `harp/frontend/tailwind.config.ts` — 把 token 注册为 Tailwind 自定义颜色
- [ ] 验证：运行 `npx tailwindcss --content "./src/**/*.tsx" --minify` 无报错

## **D2. 组件规范（原子组件）**

- [ ] `AgentStatusBadge` — props：`status: AgentStatus`，渲染对应颜色圆点 + 状态文字
- [ ] `GrowthLevelBadge` — props：`level: 1-6`，渲染等级色 + L1-L6 标签
- [ ] `AgentCard` — props：`agent`，完整 Agent 卡片（状态/等级/最近任务）
- [ ] `MemoryChip` — props：`memory`，短文本片段展示（类型标签 + truncated content）
- [ ] `TaskStatusPill` — props：`status`，任务状态胶囊标签
- [ ] `SkillTag` — props：`skill`，Skill 名称 + 类型图标

## **D3. 动效规范**

- [ ] Agent 状态切换动画：100ms ease-out（快速响应）
- [ ] Working 状态脉冲：`animate-pulse` green，周期 2s
- [ ] Evolving 状态粒子：violet 色星点扩散，Framer Motion `radial: true` 3 颗
- [ ] 成长值条进度：500ms ease-in-out，数字滚动动效
- [ ] 等级升级 Modal：1200ms celebration（confetti + 新等级大字放大淡入）
- [ ] 卡片 hover：`scale(1.01)` + `shadow-md`，150ms

## **D4. 页面布局规范**

- [ ] Agent 列表页：三列 Grid（Desktop）/ 单列（Mobile），卡片间距 24px
- [ ] Agent 详情页：左侧固定 240px 信息栏 + 右侧主内容区
- [ ] 任务时间线：垂直时序线，每条 log 64px 行高，异常条目红色左边框
- [ ] Reflection 面板：折叠卡片，默认显示 score + 一行摘要，展开显示完整内容

## **D5. 文案规范（Copy Standards）**

- [ ] Agent 状态文案：`Created / Activated / Working... / Reflecting / Evolving / Failed / Archived`（英文，全平台统一）
- [ ] 空状态文案：`此 Agent 还没有记忆` / `暂无任务` / `还没有复盘记录`
- [ ] 错误文案：`任务执行失败，Agent 已记录错误日志` / `记忆写入失败，请稍后重试`
- [ ] 成长文案：`恭喜！{name} 升级到 L{level}` / `距 L{next} 还差 {gap} 分`

---

---

# **E. 测试范例清单**

> 每条 = 一个可运行的测试用例骨架，Sprint 开发时直接填充。

---

## **E1. Agent 状态机单元测试**

```rust
// harp/backend/tests/agent_state_machine.rs
- [ ] test: Created → Activated（合法）
- [ ] test: Created → Working（非法，期望 InvalidStateTransition）
- [ ] test: Activated → Working → Reflecting → Evolving → Activated（完整复盘循环）
- [ ] test: Working → Failed（可恢复）
- [ ] test: Archived → Activated（非法）
```

```rust
// 骨架示例（直接复制入项目）
#[tokio::test]
async fn test_valid_state_transition_created_to_activated() {
    let mut agent = Agent::new_for_test("test-agent");
    assert_eq!(agent.status, AgentStatus::Created);
    
    let result = agent.transition(AgentStatus::Activated);
    assert!(result.is_ok());
    assert_eq!(agent.status, AgentStatus::Activated);
}

#[tokio::test]
async fn test_invalid_state_transition_created_to_working() {
    let mut agent = Agent::new_for_test("test-agent");
    let result = agent.transition(AgentStatus::Working);
    assert!(matches!(result, Err(AppError::InvalidStateTransition { .. })));
}
```

## **E2. Memory 写入与检索集成测试**

```rust
// harp/backend/tests/memory_integration.rs
- [ ] test: 写入 Short Memory → embedding 生成 → pgvector 存入
- [ ] test: 语义检索命中（余弦相似度 ≥ 0.80）
- [ ] test: token 预算截断（超 2000 token 的 Memory 集合截断到 2000）
- [ ] test: Working Memory 过期自动清理
- [ ] test: Memory 内容校验（XML 注入被拒绝）
```

## **E3. Skill 执行集成测试**

```rust
// harp/backend/tests/skill_execution.rs
- [ ] test: HTTP Skill 注册 → 执行 → 返回结果写入 task_logs
- [ ] test: LLM Skill 执行（Mock LLM）→ 结构化输出解析正确
- [ ] test: Skill 权限校验（deny-wins：agent 无权限被拒绝）
- [ ] test: SKILL.md 解析（合法格式）
- [ ] test: SKILL.md 解析（非法格式，missing required fields）
```

## **E4. Reflection Engine 集成测试**

```rust
// harp/backend/tests/reflection_integration.rs
- [ ] test: 任务完成事件触发 Reflection
- [ ] test: Semantic Cache 命中（第 2 次相同类型任务，0 LLM 调用）
- [ ] test: Reflection 结构化输出解析（insights/next_actions/score 字段完整）
- [ ] test: Reflection 写入 Long Memory（embedding 正确生成）
```

## **E5. Growth Engine 单元测试**

```rust
// harp/backend/tests/growth_engine.rs
- [ ] test: GrowthScore 计算公式（TaskScore×0.4 + SkillScore×0.25 + ReflectionScore×0.2 + MemoryScore×0.15）
- [ ] test: L1→L2 升级触发（score ≥ 100）
- [ ] test: 成长日志写入
- [ ] test: 成长值展示 API 响应格式正确
```

## **E6. Token 优化测试**

```rust
// harp/backend/tests/token_optimization.rs
- [ ] test: Semantic Cache 精确命中（prompt_hash 相同，0ms 返回）
- [ ] test: Semantic Cache 语义命中（余弦相似度 ≥ 0.92，返回缓存）
- [ ] test: Semantic Cache 未命中（相似度 < 0.92，调用 LLM）
- [ ] test: Memory 压缩率（compressed_tokens < original_tokens × 0.6）
- [ ] test: 过期缓存自动清理
```

## **E7. API 集成测试（E2E）**

```typescript
// harp/frontend/tests/e2e/agent-lifecycle.spec.ts（Playwright）
- [ ] test: 创建 Agent → 状态为 Created → UI 正确显示
- [ ] test: 激活 Agent → WebSocket 推送状态变更 → UI 实时更新
- [ ] test: 创建任务 → 执行 → 任务时间线更新
- [ ] test: Reflection 完成 → 成长值更新 → 等级展示正确
- [ ] test: Agent 详情页全量加载（状态/记忆/成长/最近任务）< 2s
```

---

---

# **F. 代码开发实施规划**

> 格式：`- [ ] [负责人] [P0/P1] 任务描述（预计工时）`
> 并行原则：同 Sprint 内 Backend A / Backend B / Frontend / DevOps 四条线并行

---

## **Sprint 0：基础设施 + 库集成（第 1 周）**

### **DevOps / 全栈（并行）**

- [ ] [DevOps] [P0] 建 GitHub 仓库，配置 monorepo 结构（`harp/backend/` + `harp/frontend/`）（0.5d）
- [ ] [DevOps] [P0] 写 `docker-compose.yml`：PostgreSQL 16 + pgvector + Redis 7 + NATS JetStream（0.5d）
- [ ] [DevOps] [P0] 配置 GitHub Actions CI：`cargo test` + `cargo clippy` + `cargo fmt --check` + `vitest`（1d）
- [ ] [DevOps] [P0] 配置 `.env.example`：所有必填环境变量（DATABASE_URL/REDIS_URL/NATS_URL/OPENAI_API_KEY）（0.5d）

### **Backend A（并行）**

- [ ] [BE-A] [P0] `cargo new harp-backend` 初始化 workspace，写入 `Cargo.toml` 所有依赖（见 TDD Cargo.toml）（0.5d）
- [ ] [BE-A] [P0] 跑通 `axum` Hello World + `sqlx` 连库 + `sqlx::migrate!()` 执行（0.5d）
- [ ] [BE-A] [P0] 建 `AppState` struct（PgPool + RedisPool + NatsClient + RigClient）（0.5d）
- [ ] [BE-A] [P0] 从 `adk-rust/adk-session/src/postgres.rs` 适配 advisory lock migration 模式（1d）

### **Backend B（并行）**

- [ ] [BE-B] [P0] 接入 `rig-core` 验证 LLM 调用（Hello World 级别：Claude/GPT 返回一句话）（0.5d）
- [ ] [BE-B] [P0] 接入 `swiftide` + `pgvector`：写入一条 Memory → 检索命中（0.5d）
- [ ] [BE-B] [P0] 接入 `autoagents`：注册一个 `#[tool]` + ReAct executor 跑通（0.5d）
- [ ] [BE-B] [P0] 建 `llm_response_cache` 表 + ivfflat 索引（Semantic Cache Layer 1）（1d）

### **Frontend（并行）**

- [ ] [FE] [P0] `npx create-next-app@15` 初始化，安装 shadcn/ui + Zustand + Framer Motion（0.5d）
- [ ] [FE] [P0] 建 `styles/tokens.css` — 写入所有 Design Token CSS 变量（0.5d）
- [ ] [FE] [P0] 配置 `tailwind.config.ts` 注册 Token（0.5d）
- [ ] [FE] [P0] 实现 6 个原子组件：AgentStatusBadge / GrowthLevelBadge / AgentCard / MemoryChip / TaskStatusPill / SkillTag（2d）
- [ ] [FE] [P0] 配置 Vitest + Playwright 测试框架，写第一个组件快照测试（0.5d）

### **Sprint 0 验收门**

- [ ] `cargo build` 无报错
- [ ] `cargo test` 通过（至少 5 个测试）
- [ ] swiftide 写入 pgvector + 检索命中可 demo
- [ ] LLM 调用 rig-core 可 demo
- [ ] 6 个前端原子组件 Storybook 可展示

---

## **Sprint 1：Agent Kernel（第 2-3 周）**

### **Backend A**

- [ ] [BE-A] [P0] 建数据库表：`agents` + `agent_status_logs`（完整 DDL，含索引）（0.5d）
- [ ] [BE-A] [P0] 实现 `Agent` struct + `AgentStatus` 枚举（对齐 TDD 定义）（0.5d）
- [ ] [BE-A] [P0] 实现 `Runtime` trait（7 个方法：create/load/start/stop/archive/get_status）（1d）
- [ ] [BE-A] [P0] 实现状态机 `transition()`（合法转换 + 非法拒绝 + 日志记录）（1d）
- [ ] [BE-A] [P0] 实现状态机单元测试（E1 所有 6 个测试用例）（1d）
- [ ] [BE-A] [P0] 实现 Agent API：`POST/GET/PATCH /api/agents`（1d）
- [ ] [BE-A] [P0] 实现 Agent API：`POST /api/agents/{id}/activate` + `/archive`（0.5d）
- [ ] [BE-A] [P1] 实现 Agent API：`GET /api/agents/{id}/status-logs`（0.5d）

### **Backend B（并行）**

- [ ] [BE-B] [P0] 从 `garudust-agent/crates/garudust-core/src/memory.rs` 复制并适配 `validate_memory_content`（0.5d）
- [ ] [BE-B] [P0] 从 `garudust-agent` 复制并适配 `MemoryCategory → MemoryType`（0.5d）
- [ ] [BE-B] [P1] 实现 `AppError` 枚举（对齐 TDD，含 HTTP 状态码映射）（1d）
- [ ] [BE-B] [P1] 实现 JWT 鉴权中间件（access 15min + refresh 7d）（1d）

### **Frontend（并行）**

- [ ] [FE] [P0] 实现 Agent 列表页（三列 Grid，AgentCard 组件，空状态）（1.5d）
- [ ] [FE] [P0] 实现 Agent 创建 Modal（表单：名称/角色/描述，提交调 API）（1d）
- [ ] [FE] [P0] 实现 Agent 详情页骨架（路由 `/agents/[id]`，左侧信息栏 + 右侧主区）（1d）
- [ ] [FE] [P1] 实现 Agent 状态切换按钮（激活/归档，调 API，UI 反馈）（0.5d）

### **Sprint 1 验收门**

- [ ] `POST /api/agents` 创建成功，DB 有记录
- [ ] 状态机：6 个测试用例全部通过
- [ ] 前端 Agent 列表页可从 API 加载数据并展示
- [ ] `cargo clippy` 无 warning

---

## **Sprint 2：Task Engine（第 3 周）**

### **Backend A**

- [ ] [BE-A] [P0] 建数据库表：`tasks` + `task_logs`（DDL + 索引）（0.5d）
- [ ] [BE-A] [P0] 实现 Task 实体 + TaskStatus 枚举（Pending/Running/Completed/Failed/Cancelled）（0.5d）
- [ ] [BE-A] [P0] 实现 `POST /api/tasks`（创建任务，关联 agent_id）（0.5d）
- [ ] [BE-A] [P0] 实现 `POST /api/tasks/{id}/run`（触发执行，Agent 状态 → Working）（1d）
- [ ] [BE-A] [P0] 实现 `GET /api/tasks/{id}/logs`（SSE 流式推送实时日志）（1d）
- [ ] [BE-A] [P0] 实现 `POST /api/tasks/{id}/cancel`（中止执行，Agent 状态回退）（0.5d）

### **Frontend（并行）**

- [ ] [FE] [P0] 实现任务列表面板（在 Agent 详情页右侧，Pending/Running/Completed 分组）（1d）
- [ ] [FE] [P0] 实现创建任务 Modal（标题/描述/优先级/截止时间）（0.5d）
- [ ] [FE] [P0] 实现任务执行时间线（垂直时序线，SSE 实时追加 log 条目）（1.5d）

---

## **Sprint 3：Skill Engine（第 4 周）**

### **Backend B**

- [ ] [BE-B] [P0] 建数据库表：`skills` + `agent_skills` + `skill_execution_logs`（0.5d）
- [ ] [BE-B] [P0] 从 garudust 复制 `parse_skill_md` + `Skill` struct，适配 HARP 字段（1d）
- [ ] [BE-B] [P0] 从 garudust 复制 `SkillPermissions::merge` + `sanitize_skill_name`（0.5d）
- [ ] [BE-B] [P0] 实现 `autoagents #[tool]` 注册 HARP Skill（HTTP/LLM 两种类型先跑通）（1d）
- [ ] [BE-B] [P0] 实现 Skill API：`POST /api/skills` + `GET /api/skills` + `POST /api/skills/{id}/execute`（1d）
- [ ] [BE-B] [P0] 实现 `POST /api/agents/{id}/skills/{skill_id}` 挂载 Skill（0.5d）
- [ ] [BE-B] [P0] Skill 执行集成测试（E3 全部 5 个用例）（1d）

### **Frontend（并行）**

- [ ] [FE] [P0] 实现 Skill 列表面板（在 Agent 详情页，展示已挂载 Skill）（1d）
- [ ] [FE] [P1] 实现 Skill 挂载/移除操作（调 API，即时更新）（0.5d）

---

## **Sprint 4：Memory Engine（第 5 周）**

### **Backend B**

- [ ] [BE-B] [P0] 建数据库表：`memories`（含 compressed_content / original_tokens / compressed_tokens 字段）（0.5d）
- [ ] [BE-B] [P0] 从 swiftide 适配 Memory 写入 pipeline（加 agent_id metadata + Memory TTL）（1d）
- [ ] [BE-B] [P0] 从 swiftide 适配 Memory 检索 pipeline（加 agent_id 过滤 + min_score 阈值）（1d）
- [ ] [BE-B] [P0] 从 rig-memory 适配 TokenWindowMemory（2000 token 注入预算）（1d）
- [ ] [BE-B] [P0] 实现 Memory API：`POST /api/memories` + `POST /api/memories/search` + `GET /api/agents/{id}/memories`（1d）
- [ ] [BE-B] [P0] Memory 写入/检索集成测试（E2 全部 5 个用例）（1d）

### **Frontend（并行）**

- [ ] [FE] [P0] 实现 Memory 面板（在 Agent 详情页，按分层展示 Working/Short/Long）（1.5d）
- [ ] [FE] [P1] 实现 Memory 搜索框（输入 query，调 `/api/memories/search`，展示结果）（1d）

---

## **Sprint 5：Event Bus（第 6 周前半）**

### **Backend A**

- [ ] [BE-A] [P0] NATS JetStream 连接 + Stream 创建（`HARP_EVENTS` stream，持久化）（0.5d）
- [ ] [BE-A] [P0] 定义事件类型枚举（AgentCreated/AgentActivated/TaskStarted/TaskCompleted/ReflectionTriggered/GrowthUpdated）（0.5d）
- [ ] [BE-A] [P0] 实现事件发布 `publish_event()`（序列化 + NATS publish）（0.5d）
- [ ] [BE-A] [P0] 实现事件订阅 `subscribe()`（消费者组，at-least-once）（0.5d）
- [ ] [BE-A] [P0] 建 `event_logs` 表，写入每条事件（幂等键，防重放）（0.5d）

---

## **Sprint 6：Reflection Engine（第 6 周后半）**

### **Backend B**

- [ ] [BE-B] [P0] 从 rig ExtractorAgent 适配 Reflection 结构化输出（`ReflectionOutput` struct）（0.5d）
- [ ] [BE-B] [P0] 实现 Reflection Prompt 构建（静态前缀 `cache_control` + 动态后缀）（0.5d）
- [ ] [BE-B] [P0] 接入 Semantic Response Cache（Layer 1：pgvector 相似命中）（0.5d）
- [ ] [BE-B] [P0] 实现 Reflection 消费者（订阅 `TaskCompleted` 事件 → 调用 LLM → 写 Long Memory）（1d）
- [ ] [BE-B] [P0] 建 `reflections` 表（1d）
- [ ] [BE-B] [P0] 实现 Reflection API：`GET /api/agents/{id}/reflections` + `GET /api/reflections/{id}`（0.5d）
- [ ] [BE-B] [P0] Reflection 集成测试（E4 全部 4 个用例）（1d）

---

## **Sprint 7：Growth Engine（第 7 周）**

### **Backend A**

- [ ] [BE-A] [P0] 建数据库表：`growth_profiles` + `growth_logs`（0.5d）
- [ ] [BE-A] [P0] 实现 GrowthScore 计算函数（TaskScore×0.4 + SkillScore×0.25 + ReflectionScore×0.2 + MemoryScore×0.15）（1d）
- [ ] [BE-A] [P0] 实现等级判定函数（L1-L6 阈值：0/100/300/700/1500/3000）（0.5d）
- [ ] [BE-A] [P0] 实现 Growth 事件消费者（订阅 ReflectionCompleted → 计算成长值 → 判断升级）（1d）
- [ ] [BE-A] [P0] 实现 Growth API：`GET /api/agents/{id}/growth` + `/growth/logs`（0.5d）
- [ ] [BE-A] [P0] Growth 单元测试（E5 全部 4 个用例）（1d）

### **Frontend（并行）**

- [ ] [FE] [P0] 实现 Growth 面板（成长值条 + 等级徽章 + 最近成长日志）（1.5d）
- [ ] [FE] [P0] 实现等级升级动画（1200ms celebration，Level Up Modal）（1d）

---

## **Sprint 8：HTML Agent UI（第 8-9 周）**

### **Frontend**

- [ ] [FE] [P0] 实现 WebSocket 客户端（Zustand store + 连接管理 + 断线重连）（1d）
- [ ] [FE] [P0] 实现 Agent 主页完整版（状态面板 + Skill 列表 + Memory 面板 + Growth 面板 + 任务时间线）（2d）
- [ ] [FE] [P0] 实现 Agent 状态 Working 脉冲动画（green animate-pulse）（0.5d）
- [ ] [FE] [P0] 实现 Agent 状态 Evolving 粒子效果（violet，Framer Motion）（0.5d）
- [ ] [FE] [P0] 实现 Reflection 折叠面板（score + 摘要 → 展开完整内容）（1d）
- [ ] [FE] [P0] 实现 Cost 监控面板（今日费用 / 命中率 / token 用量）（1d）
- [ ] [FE] [P1] 实现响应式适配（Mobile 单列布局）（1d）

### **Backend A（并行）**

- [ ] [BE-A] [P0] 实现 WebSocket 端点 `WS /ws/agents/{id}`（Axum WS handler）（1d）
- [ ] [BE-A] [P0] 实现 Cost 监控 API：`GET /api/agents/{id}/cost`（0.5d）

---

## **Sprint 9：集成验收（第 10 周）**

### **全员**

- [ ] [ALL] [P0] 端到端验收用例全部通过（完整闭环：创建→激活→任务→Memory→Reflection→Growth）（2d）
- [ ] [ALL] [P0] Playwright E2E 测试全部通过（E7 全部 5 个用例）（1d）
- [ ] [BE-A] [P0] 压测：100 Agent 同时运行，API P99 < 300ms（1d）
- [ ] [BE-B] [P0] Token 优化验收：Semantic Cache 命中率 ≥ 30%；单 Agent 日成本 ≤ $0.10（0.5d）
- [ ] [DevOps] [P0] 安全扫描：`cargo audit` 无高危漏洞；API 鉴权全覆盖（0.5d）
- [ ] [DevOps] [P0] 生产部署 SOP 文档化（含回滚步骤）（1d）
- [ ] [ALL] [P0] Bug 修复 + 性能调优（2d）

---

---

# **附录：Sprint 0 第一天执行命令（Copy-Ready）**

```bash
# 1. 建目录
mkdir -p ~/Projects/harp/{backend,frontend,infra}
cd ~/Projects/harp/backend

# 2. 初始化 Rust workspace
cargo init --name harp-backend
cat >> Cargo.toml << 'EOF'

[workspace]
members = ["harp-backend"]
EOF

# 3. 接入核心依赖（确认版本）
cargo add axum@0.7
cargo add tokio --features full
cargo add rig-core@0.38 --features all
cargo add rig-postgres@0.38
cargo add rig-memory@0.38
cargo add autoagents@0.32
cargo add swiftide@0.38 --features postgres,redis,openai
cargo add sqlx@0.7 --features postgres,runtime-tokio,uuid,chrono,json
cargo add async-nats@0.35
cargo add serde --features derive
cargo add serde_json
cargo add uuid --features v4,serde
cargo add chrono --features serde
cargo add thiserror
cargo add anyhow
cargo add tracing
cargo add tracing-subscriber --features env-filter
cargo add jsonwebtoken@9
cargo add tower-governor@0.4
cargo add validator@0.18 --features derive

# 4. 启动基础设施
cd ~/Projects/harp/infra
cat > docker-compose.yml << 'EOF'
services:
  postgres:
    image: pgvector/pgvector:pg16
    environment:
      POSTGRES_DB: harp
      POSTGRES_USER: harp
      POSTGRES_PASSWORD: harp_dev_2026
    ports: ["5432:5432"]
    volumes: ["pg_data:/var/lib/postgresql/data"]

  redis:
    image: redis:7-alpine
    ports: ["6379:6379"]

  nats:
    image: nats:2.10-alpine
    command: ["-js", "-m", "8222"]
    ports: ["4222:4222", "8222:8222"]

volumes:
  pg_data:
EOF
docker compose up -d

# 5. 验证
docker compose ps   # 3 个服务全 running
cargo build         # 无报错
```

---

# **附录：研究仓库路径速查**

```
~/Projects/harp/research/
├── garudust-agent/
│   ├── crates/garudust-core/src/memory.rs        ← validate_memory_content, MemoryCategory
│   ├── crates/garudust-memory/src/goal_store.rs  ← GoalStore 模式参考
│   └── crates/garudust-tools/src/toolsets/skills.rs ← parse_skill_md, WriteSkill
├── adk-rust/
│   └── adk-session/src/postgres.rs               ← advisory lock migration
├── swiftide/
│   ├── examples/index_md_into_pgvector.rs        ← Memory 写入 pipeline
│   └── examples/query_pipeline.rs               ← Memory 检索 pipeline
├── rig/
│   ├── crates/rig-memory/examples/agent_with_memory_policies.rs ← TokenWindowMemory
│   ├── crates/rig-postgres/src/lib.rs            ← PostgresVectorStore
│   └── crates/rig-core/src/extractor.rs          ← ExtractorAgent（Reflection 用）
├── AutoAgents/
│   ├── crates/autoagents-core/                   ← ReAct executor
│   └── examples/basic/                           ← #[tool] 用法示例
├── promptAgentTeam/
│   ├── SKILL.md                                  ← SKILL.md 格式规范（用于 ⑧ 模板）
│   └── references/bratko-ai-algorithms.md        ← Prolog AI 算法参考
└── RustAgentTeam/
    ├── agents/rust-backend-agent.md              ← 后端 Agent 行为规范（TDD 流程）
    ├── agents/rust-team-lead.md                  ← Hub-and-Spoke 路由 + 质量门
    ├── skills/rust-backend/SKILL.md              ← 认证/错误/数据库/DevOps 决策树
    ├── skills/rust-backend/templates/
    │   ├── axum-endpoint.rs                      ← 36 个 HARP 端点的实现模板
    │   ├── jwt-auth.rs                           ← JWT Bearer 认证（已适配加 org_id）
    │   ├── paged-list.rs                         ← Cursor 分页（Agent/Task/Memory 列表）
    │   └── test-helpers.rs                       ← Axum oneshot 测试模式
    ├── skills/rust-backend/references/
    │   ├── error-codes.md                        ← ApiResponse<T> 完整实现（含 request_id）
    │   └── bad-good-comparisons.md               ← God Command / DTO / 分页 BAD/GOOD
    └── skills/rust-arch/templates/error.rs       ← AppError 脚手架
```

---

# **附录：前期工作缺口完成状态**

| # | 缺口描述 | 状态 | 文档位置 |
|---|---------|------|---------|
| ① | 完整数据库 DDL（12张表） | ✅ 完成 | 技术规格详细.md §① |
| ② | WebSocket 消息协议 | ✅ 完成 | 技术规格详细.md §② |
| ③ | Prolog 业务规则 .pl 文件（5个） | ✅ 完成 | 技术规格详细.md §③ |
| ④ | API 完整入出参 JSON Schema | ✅ 完成 | 技术规格详细.md §④ |
| ⑤ | 项目目录结构 + .env.example | ✅ 完成 | 技术规格详细.md §⑤ |
| ⑥ | 统一错误码规范（AppError + 错误码表） | ✅ 完成 | 技术规格详细.md §⑥ |
| ⑦ | 部署方案（Railway MVP → Fly.io → K8s） | ✅ 完成 | 技术规格详细.md §⑦ |
| ⑧ | SKILL.md 标准模板 | ✅ 完成 | 技术规格详细.md §⑧ |

**所有前期工作缺口已全部填完，Sprint 0 可以启动。**
