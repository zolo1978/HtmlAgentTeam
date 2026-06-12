# HARP 项目代码评审报告

> **项目状态：** 早期骨架阶段（Sprint 0） · 无开放 PR
> **评审范围：** 全量代码库 — Rust 后端、Next.js 前端、数据库迁移、CI/CD
> **评审日期：** 2026-06-12

---

## 概览

HARP 是一个多租户 AI Agent 平台，技术选型成熟（Rust/Axum + Next.js 15 + PostgreSQL pgvector + NATS JetStream），架构规划完整。当前处于"框架已立、实现为零"的阶段——核心基础设施完成约 90%，8 个业务路由模块全部是 `// TODO` 桩代码，前端仅有 Landing 页占位，测试代码为零。

---

## 亮点

- **统一错误体系** (`error.rs`) — `thiserror` 驱动的 `AppError` 枚举、类型化错误码（E4xxx/E5xxx）、HTTP 状态码自动映射、`IntoResponse` 实现干净，前后端响应结构严格对齐，工程化程度高。
- **数据库设计** — 多租户 `org_id` 全链路传播、UUID v7 时序主键、pgvector + ivfflat ANN 索引、迁移锁（advisory lock）防并发竞争，关联删除级联配置正确。
- **CI/CD** — CI 中使用真实 pgvector 镜像，clippy `-D warnings` 强约束，`cargo fmt` 检查格式，前端 type-check + lint 全覆盖。
- **状态管理** — `AppState` + `Arc<AppConfig>` 分层设计清晰，`from_env()` 模式良好。
- **API 客户端** (`api-client.ts`) — 响应信封、错误码 i18n 映射、Bearer Token 注入设计完整。

---

## 问题与风险

### 严重（需在 Sprint 0 前修复）

#### 1. 迁移锁释放逻辑有缺陷 (`db/mod.rs:12–23`)

```rust
// 当前：如果 pg_advisory_unlock 失败，迁移错误会被吞掉
let result = sqlx::migrate!(...).run(pool).await...;
sqlx::query("SELECT pg_advisory_unlock($1)").execute(pool).await?;  // ← ? 在这里
result  // ← 如果上行 ? 返回，result 永远不会被返回
```

锁是 session 级别的，进程退出自动释放，`unlock` 失败时记录 warning 即可，不应 `?` 传播错误覆盖 `result`。建议：

```rust
if let Err(e) = sqlx::query("SELECT pg_advisory_unlock($1)")
    .bind(ADVISORY_LOCK_KEY)
    .execute(pool)
    .await
{
    tracing::warn!("advisory unlock failed (lock will auto-release): {e}");
}
result
```

#### 2. CORS 开放策略 (`api/mod.rs:39–43`)

```rust
CorsLayer::new()
    .allow_origin(Any)   // TODO: 生产环境改为具体域名
    .allow_methods(Any)
    .allow_headers(Any)
```

`TODO` 注释容易遗忘。`AppConfig` 里已有 `frontend_url` 字段，建议现在就从环境变量读取：

```rust
.allow_origin(state.config.frontend_url.parse::<HeaderValue>()?)
```

#### 3. JWT 存储在 localStorage (`api-client.ts:54`)

```ts
localStorage.getItem('harp_access_token')
```

localStorage 对 XSS 攻击完全暴露，JWT 应存在 `httpOnly` + `Secure` Cookie 中，由后端 Set-Cookie，前端不接触 token 字符串。这是认证体系的根本性安全问题，需在实现 Auth 端点前决策。

---

### 中等（Sprint 1 前修复）

#### 4. PORT 被读取两次 (`main.rs:44` vs `state.rs:43`)

`AppConfig.port` 字段存在但 `main.rs` 又独立读了一次 `PORT` 环境变量。应改为 `state.config.port`，消除不一致风险。

#### 5. LLM API Key 静默降级 (`state.rs:37–38`)

```rust
openai_api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),  // 空字符串
anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
```

未配置时静默使用空字符串，LLM 调用会报模糊的认证错误。应改为 `Option<String>`，并在 LLM 模块中早期 fail-fast。

#### 6. 数据库连接池未配置 (`state.rs:48`)

```rust
let db = sqlx::PgPool::connect(&config.database_url).await?;
```

使用默认值（max_connections=10）。生产环境应用 `PgPoolOptions`：

```rust
PgPoolOptions::new()
    .max_connections(20)
    .acquire_timeout(Duration::from_secs(3))
    .connect(&config.database_url)
    .await?
```

#### 7. `memories` 表缺少复合索引

常见查询会同时过滤 `agent_id` + `memory_type`，当前只有两个独立索引，PostgreSQL 会做 bitmap AND。建议增加：

```sql
CREATE INDEX idx_memories_agent_type ON memories(agent_id, memory_type);
```

#### 8. `agent_growth` 数据一致性风险 (`migration 005`)

`growth_score` 与子项 `task_score + skill_score + reflection_score + memory_score` 是独立字段，没有约束保证总分等于各分项之和。建议使用 generated column：

```sql
growth_score NUMERIC(10,2) GENERATED ALWAYS AS
    (task_score + skill_score + reflection_score + memory_score) STORED,
```

#### 9. `ivfflat` 小数据集效果差 (`migration 004`)

```sql
USING ivfflat (embedding vector_cosine_ops) WITH (lists = 100)
```

ivfflat 在数据量 < `lists × 39 ≈ 3900` 行时效率甚至不如顺序扫描。早期阶段建议改用 `hnsw`（对小数据集更友好）：

```sql
USING hnsw (embedding vector_cosine_ops) WITH (m = 16, ef_construction = 64)
```

#### 10. `agent_growth` 缺少 `created_at`

`agent_growth` 表只有 `updated_at`，无法追踪 Agent 何时开始成长。建议加：

```sql
created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
```

#### 11. API 错误抛出丢失类型信息 (`api-client.ts:71`)

```ts
throw new Error(msg)  // 只保留了 i18n 文字，丢失了 code 和 details
```

建议自定义错误类：

```ts
class ApiError extends Error {
  constructor(public code: string, message: string, public details?: unknown) {
    super(message)
  }
}
```

---

### 低优先级 / 建议

#### 12. 无优雅关机 (`main.rs`)

`axum::serve` 没有 SIGTERM 处理，容器环境（Docker/K8s）强杀会中断进行中的请求。建议加 `tokio::signal` 处理：

```rust
axum::serve(listener, app)
    .with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
    })
    .await?;
```

#### 13. CI 缺少 `cargo audit`

有 JWT、密码哈希、API 密钥等安全敏感依赖，建议在 CI 中加：

```yaml
- name: Audit dependencies
  run: cargo install cargo-audit && cargo audit
```

#### 14. `agents` 迁移缺少 `agent_status_logs`

文档（TDD/TECH_SPEC）提到状态变更审计追踪，但 Migration 003 只有 `agents` 表，没有 `agent_status_logs`。需要在实现状态机前补全。

#### 15. `AppState` 双层 Arc 嵌套

`Arc<AppState>` 内含 `Arc<AppConfig>`，外层 Arc 已保证共享，内层多余。可简化为 `AppState` 直接持有 `AppConfig`（`AppState` 本身已在 `Arc` 里）。

---

## 总结

| 分类 | 评分 | 说明 |
|------|------|------|
| 架构设计 | ★★★★★ | 清晰的分层和扩展路径 |
| 代码质量 | ★★★★☆ | 已写部分质量高，但量太少 |
| 安全性 | ★★★☆☆ | 有 3 个严重问题需立即解决 |
| 测试覆盖 | ★☆☆☆☆ | 0% — CI 管道空跑 |
| 完整性 | ★★☆☆☆ | 骨架完整，实现为零 |

**最高优先级行动项（开始写业务代码前必须解决）：**
1. 修复 advisory lock 错误传播逻辑 (`db/mod.rs`)
2. 将 JWT 改为 httpOnly Cookie 方案 (`api-client.ts`)
3. 将 CORS 改为从 `FRONTEND_URL` 读取白名单 (`api/mod.rs`)
