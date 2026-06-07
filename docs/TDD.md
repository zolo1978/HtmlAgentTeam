# **TDD V1.0**

---

# **1. 技术目标**

构建：

```text
Persistent Agent Runtime
```

实现：

```text
Agent 创建与激活
Agent 执行任务
Agent 调用 Skill
Agent 写入与检索 Memory
Agent 自动复盘（Reflection）
Agent 成长升级（Growth）
Agent 协作（Team）
```

---

# **2. 总体架构**

```text
┌──────────────────────────────────────────┐
│            HTML Agent UI                 │
│        (Next.js + WebSocket)             │
└──────────────────┬───────────────────────┘
                   │ HTTP / WS
                   ▼
┌──────────────────────────────────────────┐
│            Runtime Kernel                │
│     (Agent 注册 / 调度 / 生命周期)         │
└──────┬───────────┬──────────┬────────────┘
       │           │          │
       ▼           ▼          ▼
  Event Bus   Agent        Prolog
  (NATS)      Scheduler    Rule Engine
              (Tokio)      (scryer-prolog)
       │                        │
  ┌────┴──────────────┐    RuleStore
  ▼      ▼      ▼     ▼   (规则库 + Git版本)
Memory  Skill  SOP  Growth &
Engine  Engine Engine Reflection
  │       │      │    Engine
  ▼       ▼      ▼
pgvector HTTP/  DAG
(PG)    MCP    Runner
               ▼
        External Systems
        (LLM / Tools / APIs)

──────────────────────────────────────────
         两个 AgentTeam 层
──────────────────────────────────────────

RustAgentTeam              PrologAgentTeam
(构建 HARP 核心)            (构建 HARP 逻辑层)
  rust-team-lead              prolog-manager
  rust-architect              prolog-rule-designer
  rust-backend                prolog-reasoner
  rust-frontend               prolog-verifier
  rust-qa                     prolog-rust-bridge
  rust-reviewer
```

---

## **AgentTeam 分工**

### **RustAgentTeam（来自 github.com/zolo1978/RustAgentTeam）**

10 个专职 Agent，已可直接安装使用：

| **Agent** | **负责 HARP 模块** |
|---|---|
| rust-team-lead | 整体调度，质量门控 |
| rust-architect-agent | Runtime Kernel 架构设计 |
| rust-backend-agent | Memory / Skill / SOP / Growth / Reflection Engine |
| rust-frontend-agent | Next.js UI + WebSocket 集成 |
| rust-ui-designer-agent | Design Token + 组件规格 |
| rust-integration-agent | NATS Event Bus + 系统集成 |
| rust-qa-agent | 验收、反模式检测 |
| rust-reviewer | 代码审查 |
| rust-build-resolver | cargo build 错误修复 |
| rust-pm-agent | Sprint 计划对齐 PRD |

### **PrologAgentTeam（来自 github.com/zolo1978/PrologAgentTeam）**

用 Prolog 规则声明 Agent 协作逻辑，由 scryer-prolog 引擎执行：

| **Agent** | **负责 HARP 模块** |
|---|---|
| prolog-manager | 逻辑任务拆解与调度 |
| prolog-rule-designer | Agent 决策规则库、SOP 条件规则 |
| prolog-reasoner | Reflection 质量推理、Memory 一致性验证 |
| prolog-verifier | 规则一致性检查、逻辑回归测试 |
| prolog-rust-bridge | Prolog ↔ Rust 接口设计与实现 |

---

# **3. 技术栈**

## **前端**

| **模块** | **技术** | **理由** |
|---|---|---|
| Runtime UI | Next.js 15 | React 生态成熟，SSR + CSR 灵活 |
| 状态管理 | Zustand | 轻量，适合频繁状态变化的 Agent 面板 |
| Agent 实时通信 | WebSocket | Agent 状态变化需推送，非轮询 |
| UI 组件 | shadcn/ui + Tailwind | 一致性强，无样式绑架 |
| SOP 可视化 | React Flow | DAG 图形编辑，成熟方案 |
| 数据可视化 | ECharts | 成长曲线、任务统计 |
| 动效库 | Framer Motion | Agent 状态过渡、等级升级动画 |
| 图标 | Lucide React | 与 shadcn/ui 默认一致 |
| 骨架屏 | shadcn/ui Skeleton | 替代 loading spinner |

---

## **Design Token 系统**

所有视觉属性通过 Tailwind CSS 变量统一管理，禁止硬编码颜色/间距。

### **颜色 Token**

```css
/* Agent 状态色（语义色）*/
--color-agent-created:    #6B7280;  /* gray-500 */
--color-agent-activated:  #3B82F6;  /* blue-500 */
--color-agent-working:    #10B981;  /* emerald-500 */
--color-agent-reflecting: #F59E0B;  /* amber-500 */
--color-agent-evolving:   #8B5CF6;  /* violet-500 */
--color-agent-failed:     #EF4444;  /* red-500 */
--color-agent-archived:   #D1D5DB;  /* gray-300 */

/* 等级色 */
--color-level-l1: #9CA3AF;  /* gray */
--color-level-l2: #34D399;  /* green */
--color-level-l3: #60A5FA;  /* blue */
--color-level-l4: #A78BFA;  /* violet */
--color-level-l5: #F59E0B;  /* gold */
--color-level-l6: #F97316;  /* orange-red（大师）*/
```

### **间距规则**

- 基础单位：4px（Tailwind `space-1`）
- 组件内间距：8px / 12px / 16px
- 卡片间距：24px
- 页面边距：32px（Desktop）/ 16px（Mobile）
- 禁止出现非 4 倍数的间距

### **字体规范**

```text
页面标题：text-2xl font-bold      （24px，Agent 名称）
卡片标题：text-lg font-semibold   （18px，模块标题）
正文：    text-sm font-normal     （14px，列表内容）
辅助文字：text-xs text-muted-foreground（12px，时间、来源）
```

### **圆角规范**

```text
卡片：    rounded-xl  （12px）
按钮：    rounded-md  （6px）
标签/Badge：rounded-full
输入框：  rounded-md
```

---

## **组件架构**

### **分层原则**

```text
pages/          — 页面级组件（路由对应）
components/
  ├── ui/       — 原子组件（shadcn/ui 扩展）
  ├── agent/    — Agent 领域组件（AgentCard、AgentStatusBadge）
  ├── task/     — Task 领域组件（TaskTimeline、TaskStatusTag）
  ├── memory/   — Memory 领域组件（MemoryList、MemorySearch）
  ├── growth/   — Growth 领域组件（GrowthBar、LevelBadge）
  └── layout/   — 布局组件（Sidebar、PageHeader）
```

### **Agent 状态核心组件**

```tsx
// AgentStatusBadge — 全局统一，禁止各处自定义颜色
interface AgentStatusBadgeProps {
  status: AgentStatus;
  showPulse?: boolean;  // Working 状态显示呼吸动画
}

// AgentCard — 列表页卡片
interface AgentCardProps {
  agent: Agent;
  onClick: () => void;
}

// GrowthProgressBar — 成长进度条
interface GrowthProgressBarProps {
  score: number;
  level: AgentLevel;
  showMilestone?: boolean;
}
```

---

## **动效规范**

### **动效原则**

- 目的性：每个动效有明确意图（状态变化、引导注意、庆祝成就）
- 克制性：大多数操作无动效，动效只用于关键时刻
- 性能优先：优先使用 CSS transform/opacity，避免触发重排

### **标准时长**

```text
微交互（按钮 hover/press）：100-150ms，ease-out
状态切换（badge 颜色变化）：200-300ms，ease-in-out
卡片进入（列表加载）：300ms，ease-out（stagger 50ms/item）
等级升级动画：800-1200ms，spring 弹簧曲线
```

### **关键动效定义**

```text
Working 状态呼吸灯：
  badge 圆点 scale: 1.0 → 1.4 → 1.0，2s 循环，opacity 同步

Agent 等级升级：
  1. 当前 badge 放大 scale: 1 → 1.3
  2. 金色粒子从 badge 扩散（canvas 动画）
  3. 新等级数字 flip-in（translateY: -20px → 0）
  4. 全屏遮罩淡入淡出（可 Esc 跳过）
  持续：1200ms

Reflection 流式输出：
  文字逐字打出，速度 40 字/秒，光标闪烁
  背景色从 amber-50 渐变到 white

任务完成 Toast：
  从右上角 slideIn，停留 3s，fadeOut
  成功：emerald 主题 + 勾号图标
  失败：red 主题 + 感叹号图标
```

---

## **实时数据更新模式**

### **WebSocket 消息处理**

```typescript
// Zustand store 消费 WebSocket 推送
interface AgentStore {
  agents: Record<string, Agent>;
  handleAgentStatusChanged: (agentId: string, status: AgentStatus) => void;
  handleTaskUpdated: (task: Task) => void;
  handleGrowthUpdated: (agentId: string, growth: GrowthProfile) => void;
}

// 状态变更时触发对应动效
const handleAgentStatusChanged = (agentId, status) => {
  set(state => ({ agents: { ...state.agents, [agentId]: { ...state.agents[agentId], status } } }));
  if (status === 'evolving') triggerLevelUpAnimation(agentId);
  if (status === 'working') startPulseAnimation(agentId);
};
```

### **乐观更新规则**

- 创建 / 更新操作：立即乐观更新 UI，请求失败后回滚 + Toast 提示
- 状态转换：等待 WebSocket 确认后更新（不乐观，避免状态闪烁）
- 删除操作：二次确认弹窗 + 乐观删除 + 失败回滚

---

## **后端**

| **模块** | **技术** | **来源** | **理由** |
|---|---|---|---|
| API Server | Rust (Axum 0.7) | 自研 | 高并发，内存安全，零成本抽象 |
| Runtime Kernel | Rust (Tokio) | 自研 | 异步调度，适合 Agent 长生命周期 |
| Agent Scheduler | Rust (Tokio) | 自研 | 并发管理 1000+ Agent 状态 |
| Event Bus | NATS (async-nats) | 自研接入 | 轻量，高吞吐，模块解耦 |
| SOP Runner | Rust | 自研 | DAG 节点执行，需精确控制 |
| **LLM 客户端** | **rig 0.36** | **cargo add** | 20+ provider 开箱即用，pgvector 内置，替代手写 HTTP client |
| **Agent 执行引擎** | **autoagents** | **cargo add** | ReAct executor + Tool/Skill 系统 + pub/sub，替代手写调度 |
| **Memory RAG 管道** | **swiftide** | **cargo add** | 流式向量索引 + 语义检索，替代手写 embedding pipeline |
| **Prolog 规则引擎** | **scryer-prolog 0.9** | **PrologAgentTeam 代码** | 纯 Rust ISO Prolog，嵌入主进程，无需独立服务 |

**为什么全 Rust 后端：**
Agent 天生是高并发、高状态、长生命周期的场景。Rust 的 async/await + Tokio 在这个场景下性能领先，且内存安全避免生产事故。

---

## **核心依赖 Cargo.toml**

```toml
[workspace]
members = ["backend"]

[dependencies]
# === 核心框架 ===
axum            = "0.7"
tokio           = { version = "1", features = ["full"] }
async-trait     = "0.1"

# === LLM 层（替代手写 HTTP client）===
# 实际版本：rig workspace = 0.38.1
rig-core        = { version = "0.38", features = ["all"] }
rig-postgres    = "0.38"   # PostgresVectorStore + PgVectorDistanceFunction::Cosine
rig-memory      = "0.38"   # TokenWindowMemory（直接实现 2000 token 预算）+ SlidingWindowMemory
# 支持 OpenAI / Claude / Mistral / 火山方舟 / Ollama 20+ provider，无需手写 HTTP

# === Agent 执行层（替代手写调度器）===
# 实际版本：autoagents 0.32.1
autoagents      = "0.32"
# 内置：ReAct executor、#[tool] 宏、SlidingWindowMemory、pub/sub Agent 通信

# === Memory RAG 层（替代手写 embedding pipeline）===
# 实际版本：swiftide 0.38.1（workspace 版本）
swiftide        = { version = "0.38", features = ["postgres", "redis", "openai"] }
# indexing::Pipeline + query::Pipeline + pgvector persist/retrieve 直接可用

# === Prolog 规则引擎（来自 PrologAgentTeam）===
scryer-prolog   = "0.9"

# === 数据层 ===
sqlx            = { version = "0.7", features = ["postgres", "runtime-tokio", "uuid", "chrono", "json"] }
redis           = { version = "0.24", features = ["tokio-comp"] }

# === Event Bus ===
async-nats      = "0.35"

# === 序列化 ===
serde           = { version = "1", features = ["derive"] }
serde_json      = "1"

# === 验证 / 错误处理 ===
validator       = { version = "0.18", features = ["derive"] }
thiserror       = "1"
anyhow          = "1"

# === 工具 ===
uuid            = { version = "1", features = ["v4", "serde"] }
chrono          = { version = "0.4", features = ["serde"] }
tracing         = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# === 安全 / 限流 ===
tower-governor  = "0.4"
jsonwebtoken    = "9"
```

---

## **模块归属：库 vs 自研（源码核实版）**

> 以下结论基于对 garudust-agent / adk-rust / swiftide / rig / AutoAgents 五个仓库源码的实际阅读。

| **模块** | **实现方式** | **核心库 + 关键路径** | **自研部分** |
|---|---|---|---|
| LLM 客户端 | **直接用库** | `rig-core` `crates/rig-core/src/` | 仅写 HARP 业务 Prompt 模板，打 cache_control |
| Agent 执行引擎 | **直接用库** | `autoagents` `crates/autoagents-core/` — ReAct executor + #[tool] 宏 + pub/sub | 注册 HARP Skill 为 Tool，接入状态机事件 |
| Memory RAG — 写入 | **直接用库** | `swiftide` `swiftide-indexing/src/pipeline.rs` — `Pipeline::from_loader→then_chunk→then_in_batch(Embed)→then_store_with(PgVector)` | 自定义 ChunkStrategy（Memory 分层 TTL），追加 compression 步骤 |
| Memory RAG — 检索 | **直接用库** | `swiftide` `swiftide-query/src/query/pipeline.rs` — `GenerateSubquestions→Embed→retrieve→Summary→answer` | 配置 top_k 和 min_score 阈值（分层预算） |
| Memory Token 预算 | **直接用库** | `rig-memory` `crates/rig-memory/` — `TokenWindowMemory` + `SlidingWindowMemory` | 把 2000 token 硬上限接入 HARP 注入逻辑 |
| pgvector 向量检索 | **直接用库** | `rig-postgres` `crates/rig-postgres/src/lib.rs` — `PostgresVectorStore` + `PgVectorDistanceFunction::Cosine` | 建表 DDL、配置 ivfflat 索引 |
| Session 持久化 | **直接用库** | `adk-rust` `adk-session/src/postgres.rs` — advisory lock + JSONB 三层 state（app/user/session）| HARP 改为 agent/org/system 三层 |
| Skill SKILL.md 解析 | **复制适配** | `garudust-agent` `crates/garudust-tools/src/toolsets/skills.rs` — `parse_skill_md` + `load_skills_from_dir` | 改为 HARP Skill frontmatter（加 skill_type / input_schema / output_schema 字段） |
| Skill 权限控制 | **复制适配** | `garudust-agent` `crates/garudust-core/src/memory.rs` — `SkillPermissions::merge()` deny-wins 语义 | 接入 HARP RBAC 角色体系 |
| Memory 内容校验 | **复制适配** | `garudust-agent` `crates/garudust-core/src/memory.rs` — `validate_memory_content`（XML 注入防御，500字符限制）| 调整字符上限 + 加 HARP 的 Memory 分层校验 |
| Prolog 规则引擎 | **复制代码** | PrologAgentTeam `src/scryer_runtime.rs` + `rule_store.rs` + `git_manager.rs` | 编写 5 类 HARP 业务规则（任务分配/等级/SOP/记忆/成长）|
| Agent 状态机 | **完全自研** | — | 7 状态转换、非法转换拒绝、日志、事件发布 |
| Runtime Kernel | **完全自研** | — | Agent 注册、调度、生命周期管理 |
| Growth Engine | **完全自研** | — | GrowthScore 公式、L1-L6 等级、GrowthProfile |
| Reflection Engine | **完全自研** | — | 自动触发、Prompt 结构（静态前缀+动态后缀）、结构化输出 |
| SOP Runner | **完全自研** | — | DAG 节点执行器、条件跳转、异常恢复 |
| Semantic Cache | **完全自研** | pgvector（已有）| `llm_response_cache` 表 + 精确/语义双路命中 |
| Event Bus 接入 | **自研接入** | `async-nats` | 事件持久化、订阅分发 |
| API Server | **自研接入** | `axum` | REST 路由、JWT 中间件、rate limiting |
| HTML Agent UI | **完全自研** | Next.js + shadcn | Agent 主页、状态面板、动效 |

### **节省估算（基于源码实际体量）**

| **来源** | **替代的自研工作** | **节省周数** |
|---|---|---|
| rig-core + rig-postgres | LLM client + pgvector 封装 | 2 周 |
| autoagents | ReAct loop + Tool 注册系统 + pub/sub | 2 周 |
| swiftide | Embedding pipeline + 语义检索 pipeline | 2 周 |
| rig-memory | TokenWindowMemory（Token 预算已有现成实现）| 0.5 周 |
| garudust: parse_skill_md | SKILL.md 格式解析 + 权限控制 | 0.5 周 |
| garudust: validate_memory_content | Memory 安全校验 | 0.5 周 |
| adk-rust: postgres.rs | Session 持久化 + advisory lock migration | 1 周 |
| **合计** | | **≈ 8.5 周** |

**关键结论：** 五个仓库真正能抽出来用的约节省 **8.5 周**，MVP 从 17 周压缩到约 **8-9 周**。

---

## **Agent 推理层**

| **模块** | **技术** | **说明** |
|---|---|---|
| LLM 调用 | rig-core（OpenAI / Claude / Ollama 等） | 20+ provider，开箱即用，无需手写 HTTP |
| Reflection 生成 | rig + 结构化 Prompt | 任务结束后调用 LLM 生成复盘 |
| Agent 执行 | autoagents ReAct executor | 替代手写 ReAct 循环，直接注册 Tool |
| Memory 向量化 | swiftide embedding pipeline | 流式写入 pgvector，支持批量 |
| Prolog Rule Engine | **scryer-prolog（Rust crate）** | **V1，MVP 集成，嵌入主进程无需独立服务** |
| 规则版本管理 | git_manager.rs（来自 PrologAgentTeam）| 规则库版本化，支持回滚 |

---

## **数据层**

| **模块** | **技术** | **说明** |
|---|---|---|
| 主数据库 | PostgreSQL 16 | 所有业务数据 |
| 缓存 | Redis 7 | Agent 状态热缓存，Session |
| 向量存储 | pgvector | Memory 语义检索，嵌入维度 1536（OpenAI text-embedding-3-small）|
| 对象存储 | MinIO | Skill 产物、日志附件 |

---

# **4. 核心内核设计**

---

## **Agent Kernel**

Agent 核心实体，代表一个持久化的 Agent 实例。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub role: String,
    pub status: AgentStatus,
    pub owner_id: Uuid,
    pub organization_id: Uuid,
    pub memory_id: Uuid,
    pub skill_ids: Vec<Uuid>,
    pub sop_ids: Vec<Uuid>,
    pub growth_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentStatus {
    Created,
    Activated,
    Working,
    Reflecting,
    Evolving,
    Failed,
    Archived,
}
```

职责：

```text
生命周期管理（状态机转换）
任务接收与委派
事件发布（通过 Event Bus）
状态持久化
```

---

## **Runtime Kernel**

整个系统的调度核心，管理所有 Agent 实例的生命周期。

```rust
#[async_trait]
pub trait Runtime: Send + Sync {
    /// 创建并持久化一个新 Agent
    async fn create_agent(&self, req: CreateAgentRequest) -> Result<Agent, RuntimeError>;

    /// 从数据库加载 Agent 到内存
    async fn load_agent(&self, agent_id: Uuid) -> Result<Agent, RuntimeError>;

    /// 激活 Agent，状态变更为 Activated
    async fn start_agent(&self, agent_id: Uuid) -> Result<(), RuntimeError>;

    /// 暂停 Agent 执行
    async fn stop_agent(&self, agent_id: Uuid) -> Result<(), RuntimeError>;

    /// 归档 Agent，状态变更为 Archived
    async fn archive_agent(&self, agent_id: Uuid) -> Result<(), RuntimeError>;

    /// 查询 Agent 当前状态
    async fn get_agent_status(&self, agent_id: Uuid) -> Result<AgentStatus, RuntimeError>;
}
```

---

## **状态机转换规则**

合法转换：

```text
Created    → Activated
Activated  → Working
Working    → Reflecting
Working    → Failed
Reflecting → Evolving
Evolving   → Activated  （循环：完成一轮后重新就绪）
Failed     → Activated  （恢复）
任意状态   → Archived   （终态）
```

非法转换一律拒绝并返回 `InvalidStateTransition` 错误，同时写入状态变更日志。

---

# **5. Memory Engine**

---

## **设计目标**

支持四层记忆，语义检索，Agent 间权限隔离。

---

## **数据结构**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub organization_id: Uuid,
    pub memory_type: MemoryType,
    pub content: String,
    pub source: MemorySource,
    pub embedding: Vec<f32>,        // 维度 1536，OpenAI text-embedding-3-small
    pub metadata: serde_json::Value,
    pub expires_at: Option<DateTime<Utc>>,  // Working: 任务结束; Short: +30天; Long: None
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    Working,
    Short,
    Long,
    Organization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemorySource {
    Task,
    Reflection,
    User,
    Sop,
}
```

---

## **Memory Flow**

```text
任务执行
  ↓
产生执行结果
  ↓
Reflection Engine 分析
  ↓
Memory Engine 写入（带 embedding）
  ↓
pgvector 向量索引
  ↓
下次任务时语义检索，注入上下文
```

---

## **权限隔离规则**

- `Working` / `Short` / `Long` Memory：只能被归属 `agent_id` 的 Agent 访问。
- `Organization` Memory：同 `organization_id` 下所有 Agent 可读，Organization Admin 可写。
- 搜索接口强制携带 `agent_id`，后端过滤，不暴露跨 Agent 数据。

---

# **6. Skill Engine**

---

## **数据结构**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub skill_type: SkillType,
    pub endpoint: String,
    pub version: String,
    pub status: SkillStatus,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillType {
    Builtin,
    Http,
    Mcp,
    Rust,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillStatus {
    Enabled,
    Disabled,
}
```

---

## **Skill 生命周期**

```text
Register（注册）
  ↓
Activate（启用）
  ↓
Execute（调用）
  ↓
Monitor（指标采集）
  ↓
Upgrade / Rollback（版本管理）
```

---

## **执行接口**

```rust
#[async_trait]
pub trait SkillExecutor: Send + Sync {
    async fn execute(
        &self,
        skill_id: Uuid,
        agent_id: Uuid,
        task_id: Uuid,
        input: serde_json::Value,
    ) -> Result<SkillOutput, SkillError>;
}

pub struct SkillOutput {
    pub success: bool,
    pub output: serde_json::Value,
    pub duration_ms: u64,
}
```

---

# **7. SOP Engine**

---

## **设计目标**

把经验固化为可自动执行的有向图工作流：

```text
经验 → SOP 定义 → 自动执行 → 复盘改进 → SOP 迭代
```

---

## **节点数据结构**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopNode {
    pub id: Uuid,
    pub node_type: SopNodeType,
    pub name: String,
    pub config: serde_json::Value,   // 每种节点类型的具体配置
    pub next_nodes: Vec<Uuid>,       // 条件节点可有多个出边
    pub on_error: Option<Uuid>,      // 异常跳转节点
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SopNodeType {
    Start,
    Action,
    SkillCall,
    Condition,
    Loop,
    MemoryWrite,
    Reflection,
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sop {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub version: String,
    pub nodes: Vec<SopNode>,
    pub edges: Vec<SopEdge>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopEdge {
    pub from: Uuid,
    pub to: Uuid,
    pub condition: Option<String>,   // 条件表达式，None 表示无条件
}
```

---

## **SOP Runner**

```rust
#[async_trait]
pub trait SopRunner: Send + Sync {
    async fn run(
        &self,
        sop_id: Uuid,
        agent_id: Uuid,
        task_id: Uuid,
        context: serde_json::Value,
    ) -> Result<SopRunResult, SopError>;
}
```

执行逻辑：从 `Start` 节点开始，按 DAG 拓扑顺序执行，每个节点结果写入执行日志，异常时跳转 `on_error` 节点或直接中止。

---

# **8. Prolog Rule Engine（V1）**

---

## **为什么 Prolog，不是 if/else**

```text
if/else 写 Agent 协作规则：
  代码量爆炸，规则越加越乱，无法回溯

Prolog 声明 Agent 协作规则：
  agent_can_handle(memory_task, agent) :-
      agent_skill(agent, vector_search),
      agent_status(agent, activated).

  规则即文档，增量可组合，天然支持回溯推理
```

---

## **技术选型：scryer-prolog**

来源：PrologAgentTeam（github.com/zolo1978/PrologAgentTeam）已验证

```toml
# Cargo.toml
[dependencies]
scryer-prolog = "0.9"
```

scryer-prolog 是纯 Rust 实现的 ISO Prolog 引擎，直接作为 crate 嵌入 HARP 主进程，无需 SWI-Prolog 独立服务。

---

## **复用 PrologAgentTeam 的代码**

| **文件** | **复用方式** | **用于 HARP** |
|---|---|---|
| `scryer_runtime.rs` | 直接复用 | HARP Prolog Engine 核心 |
| `rule_store.rs` | 直接复用 | HARP 规则库加载与查询 |
| `git_manager.rs` | 直接复用 | SOP 规则版本管理 |
| `feedback_loop.rs` | 参考复用 | Reflection Engine 闭环逻辑 |
| `llm_client.rs` | 参考复用 | HARP LLM 客户端封装 |

---

## **Prolog 规则负责的决策**

```prolog
% 1. Agent 任务分配规则
can_assign(Task, Agent) :-
    task_type(Task, Type),
    agent_skill(Agent, Type),
    agent_status(Agent, activated),
    \+ agent_overloaded(Agent).

% 2. AgentTeam 角色选择规则
team_role(Task, manager) :- task_complexity(Task, high).
team_role(Task, worker)  :- task_complexity(Task, low).

% 3. SOP 条件分支规则
sop_next_node(Node, NextNode) :-
    node_condition(Node, Cond),
    evaluate_condition(Cond, true),
    node_on_success(Node, NextNode).

% 4. Memory 一致性验证
memory_consistent(NewMem) :-
    \+ contradicts_existing(NewMem).

% 5. Growth 等级触发规则
should_level_up(Agent, NewLevel) :-
    agent_score(Agent, Score),
    level_threshold(NewLevel, Min),
    Score >= Min,
    agent_level(Agent, CurrentLevel),
    level_above(NewLevel, CurrentLevel).
```

---

## **Prolog Engine 数据结构**

```rust
// 来自 PrologAgentTeam rule_store.rs，适配 HARP
pub struct RuleStore {
    pub rules: Vec<PrologRule>,
    pub facts: Vec<PrologFact>,
    pub version: String,          // Git commit hash
}

pub struct ScryerRuntime {
    machine: Machine,             // scryer-prolog Machine
    rule_store: Arc<RuleStore>,
}

impl ScryerRuntime {
    pub async fn query(&self, goal: &str) -> Result<Vec<Solution>, PrologError>;
    pub async fn assert_fact(&self, fact: &str) -> Result<(), PrologError>;
    pub async fn retract_fact(&self, fact: &str) -> Result<(), PrologError>;
}
```

---

# **9. Growth Engine**

这是整个系统最特殊的部分，也是目前全球 Agent 产品最缺失的能力。

---

## **成长公式**

```text
GrowthScore =
  TaskScore     × 0.40
+ SkillScore    × 0.25
+ ReflectionScore × 0.20
+ MemoryScore   × 0.15
```

**权重依据：**

| **维度** | **权重** | **理由** |
|---|---|---|
| TaskScore | 40% | 任务完成是最直接的产出，核心指标 |
| SkillScore | 25% | 技能掌握度是能力积累的直接体现 |
| ReflectionScore | 20% | 反思质量决定学习速度，高权重激励深度复盘 |
| MemoryScore | 15% | 记忆积累是能力的底层支撑，效果间接 |

---

## **各子分计算规则**

```text
TaskScore      = 完成任务数 × 任务难度系数（1-3） × 成功质量系数（0.5-1.0）
SkillScore     = Σ(Skill调用次数 × 成功率)，上限100
ReflectionScore = Reflection 数量 × LLM 评分（0-5分标准化）
MemoryScore    = Long Memory 写入条数 × 0.5
```

---

## **等级系统**

```text
L1 新手   :    0 -   99 分
L2 初级   :  100 -  299 分
L3 中级   :  300 -  699 分
L4 高级   :  700 - 1499 分
L5 专家   : 1500 - 2999 分
L6 大师   : 3000+    分
```

等级升级后：发布 `AgentEvolved` 事件，触发技能树解锁检查。

---

## **数据结构**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthProfile {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub total_score: f64,
    pub level: AgentLevel,
    pub task_score: f64,
    pub skill_score: f64,
    pub reflection_score: f64,
    pub memory_score: f64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentLevel { L1, L2, L3, L4, L5, L6 }
```

---

# **9. Reflection Engine**

---

## **触发机制**

由 Event Bus 消费以下事件后异步触发，不阻塞任务执行流程：

```text
TaskCompleted
TaskFailed
SopFinished
SkillFailed（连续失败 3 次）
```

---

## **执行流程**

```text
Event Bus 消费事件
  ↓
加载任务上下文（目标、步骤日志、Skill 调用记录）
  ↓
调用 LLM（结构化 Prompt）
  ↓
解析输出为 Reflection 结构体
  ↓
写入 reflections 表
  ↓
写入 Long Memory（embedding 存储）
  ↓
发布 ReflectionCreated 事件
  ↓
Growth Engine 消费，更新 ReflectionScore
```

---

## **输出结构**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub task_id: Uuid,
    pub success: bool,
    pub summary: String,
    pub root_cause: Option<String>,
    pub improvement: String,
    pub lessons: String,
    pub created_at: DateTime<Utc>,
}
```

---

# **10. Event Bus**

---

## **设计原则**

所有模块禁止直接调用彼此接口，统一通过 Event Bus 通信。

选型：**NATS**（轻量、高吞吐、支持 at-least-once 语义）

---

## **事件类型**

```text
AgentCreated
AgentActivated
AgentArchived
AgentEvolved
TaskCreated
TaskStarted
TaskCompleted
TaskFailed
TaskCancelled
SkillExecuted
SkillFailed
MemoryWritten
ReflectionCreated
GrowthUpdated
SopStarted
SopFinished
```

---

## **事件数据结构**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub agent_id: Uuid,
    pub task_id: Option<Uuid>,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
```

所有事件持久化到 `events` 表，支持回放和审计。

---

# **11. Agent Team**

---

## **团队模式**

```text
Manager Agent   — 接收用户任务，拆解并分配给 Planner
Planner Agent   — 制定执行计划，分配给 Workers
Worker Agent    — 执行具体子任务
Reviewer Agent  — 检验 Worker 输出，决定是否通过
```

---

## **调度逻辑**

```text
用户任务
  ↓
Manager Agent 拆解
  ↓ Event Bus
Planner Agent 制定计划
  ↓ Event Bus
Workers 并行执行
  ↓ Event Bus
Reviewer Agent 审核
  ↓ Event Bus
Memory Engine 写入团队结果
```

Agent 之间不直接调用，全部通过 Event Bus 消息驱动。

---

# **12. API 设计**

---

## **Agent API**

```http
POST   /api/agents
GET    /api/agents
GET    /api/agents/{id}
PATCH  /api/agents/{id}
DELETE /api/agents/{id}
POST   /api/agents/{id}/activate
POST   /api/agents/{id}/archive
```

---

## **Task API**

```http
POST /api/tasks
GET  /api/tasks
GET  /api/tasks/{id}
POST /api/tasks/{id}/run
POST /api/tasks/{id}/cancel
GET  /api/tasks/{id}/logs
```

---

## **Memory API**

```http
POST /api/memories                  # 写入，需 agent_id
POST /api/memories/search           # 语义检索，强制 agent_id 过滤
GET  /api/agents/{id}/memories      # 按 Agent 列表
```

---

## **Skill API**

```http
POST /api/skills
GET  /api/skills
POST /api/skills/{id}/enable
POST /api/skills/{id}/disable
POST /api/skills/{id}/execute
GET  /api/skills/{id}/logs
```

---

## **SOP API**

```http
POST  /api/sops
GET   /api/sops
GET   /api/sops/{id}
PATCH /api/sops/{id}
POST  /api/sops/{id}/run
GET   /api/sops/{id}/runs           # 执行历史
```

---

## **Reflection API**

```http
GET  /api/agents/{id}/reflections
POST /api/reflections               # 手动触发（测试用）
GET  /api/reflections/{id}
```

---

## **Growth API**

```http
GET /api/agents/{id}/growth
GET /api/agents/{id}/growth/logs
```

---

# **13. 数据库设计**

## **核心表**

```text
users
organizations
agents
agent_status_logs
tasks
task_logs
skills
agent_skills              — Agent 与 Skill 的关联表
skill_execution_logs
sops
sop_versions
sop_execution_logs
memories                  — 含 embedding vector(1536) 列
reflections
growth_profiles
growth_logs
events
```

---

## **关键索引**

```sql
-- Memory 向量检索
CREATE INDEX ON memories USING ivfflat (embedding vector_cosine_ops);

-- Agent 状态查询
CREATE INDEX ON agents (status, organization_id);

-- 任务查询
CREATE INDEX ON tasks (agent_id, status, created_at DESC);

-- 事件溯源
CREATE INDEX ON events (agent_id, event_type, created_at DESC);
```

---

# **14. 部署架构**

## **MVP 阶段**

```text
Docker Compose（本地开发）
  ↓
K8S（Staging / Production）
  ↓
Cloud（AWS / GCP）
```

---

## **服务拆分**

```text
agent-service       — Agent CRUD + 状态机
task-service        — 任务管理 + 执行
skill-service       — Skill 注册 + 执行
sop-service         — SOP 定义 + 运行
memory-service      — Memory 写入 + 检索
reflection-service  — 复盘生成
growth-service      — 成长值计算
event-service       — Event Bus 持久化
```

---

# **14. 安全设计**

---

## **认证与授权**

### **认证方案**

```text
认证方式：JWT（Access Token + Refresh Token）
Access Token 有效期：15 分钟
Refresh Token 有效期：7 天（单次使用，用后作废）
存储：Access Token 存内存，Refresh Token 存 HttpOnly Cookie
```

### **JWT Payload 结构**

```json
{
  "sub": "user_uuid",
  "org_id": "org_uuid",
  "role": "member | owner | org_admin | super_admin",
  "exp": 1234567890
}
```

### **鉴权中间件（Axum）**

```rust
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_bearer_token(&req)?;
    let claims = verify_jwt(&token, &state.jwt_secret)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}
```

---

## **权限控制（RBAC）**

### **资源操作矩阵**

| **资源** | **Super Admin** | **Org Admin** | **Owner** | **Member** |
|---|---|---|---|---|
| 创建 Agent | ✅ | ✅ | ✅ | ❌ |
| 查看他人 Agent | ✅ | ✅（同组织）| ❌ | ❌ |
| 修改他人 Agent | ✅ | ✅（同组织）| ❌ | ❌ |
| 分配任务 | ✅ | ✅ | ✅ | ✅（被授权 Agent）|
| 读取 Memory | ✅ | ✅（同组织）| ✅（自己）| ❌ |
| 读取 Organization Memory | ✅ | ✅ | ✅ | ✅（只读）|
| 管理 Skill | ✅ | ✅ | ✅（自己）| ❌ |

### **资源归属校验（所有写操作必须执行）**

```rust
pub fn assert_agent_owner(claims: &Claims, agent: &Agent) -> Result<(), AppError> {
    if claims.role == Role::SuperAdmin { return Ok(()); }
    if claims.role == Role::OrgAdmin && claims.org_id == agent.organization_id { return Ok(()); }
    if agent.owner_id == claims.sub { return Ok(()); }
    Err(AppError::Forbidden("无权操作此 Agent".into()))
}
```

---

## **API 安全**

### **Rate Limiting**

```text
全局限流：100 req/min/IP（未登录）
认证用户：500 req/min/user
Skill 执行：20 req/min/agent（防止滥用 LLM 调用）
Memory 检索：60 req/min/agent
```

实现：`tower-governor` 中间件，基于 Redis 滑动窗口计数器。

### **输入验证**

```rust
// 所有请求体使用 validator crate 自动校验
#[derive(Deserialize, Validate)]
pub struct CreateAgentRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,

    #[validate(length(max = 2000))]
    pub description: Option<String>,

    #[validate(length(min = 1, max = 255))]
    pub role: String,
}
```

### **敏感数据保护**

- LLM API Key 存储：加密存储于数据库（AES-256-GCM），不在日志中打印
- Memory 内容：不在 API 响应中返回 embedding 向量（仅返回文本 + score）
- 错误响应：生产环境不暴露堆栈信息，只返回 error_code + 用户友好 message

---

## **错误类型体系**

### **统一错误枚举**

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // 业务错误
    #[error("资源不存在: {0}")]
    NotFound(String),

    #[error("无权访问: {0}")]
    Forbidden(String),

    #[error("非法状态转换: {from:?} → {to:?}")]
    InvalidStateTransition { from: AgentStatus, to: AgentStatus },

    #[error("Skill 执行失败: {0}")]
    SkillExecutionFailed(String),

    #[error("Memory 写入失败: {0}")]
    MemoryWriteFailed(String),

    // 基础设施错误
    #[error("数据库错误")]
    Database(#[from] sqlx::Error),

    #[error("Redis 错误")]
    Cache(#[from] redis::RedisError),

    #[error("LLM 调用失败: {0}")]
    LlmError(String),

    #[error("向量检索失败: {0}")]
    VectorSearchFailed(String),

    // 通用错误
    #[error("输入验证失败: {0}")]
    Validation(String),

    #[error("内部错误")]
    Internal(#[from] anyhow::Error),
}
```

### **HTTP 错误响应映射**

```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::NotFound(msg)              => (404, "NOT_FOUND", msg.clone()),
            AppError::Forbidden(msg)             => (403, "FORBIDDEN", msg.clone()),
            AppError::InvalidStateTransition{..} => (400, "INVALID_STATE", self.to_string()),
            AppError::Validation(msg)            => (422, "VALIDATION_ERROR", msg.clone()),
            AppError::SkillExecutionFailed(msg)  => (502, "SKILL_FAILED", msg.clone()),
            AppError::LlmError(_)                => (502, "LLM_ERROR", "AI 服务暂时不可用".into()),
            _                                    => (500, "INTERNAL_ERROR", "服务内部错误".into()),
        };

        let body = json!({ "error": { "code": code, "message": message } });
        (StatusCode::from_u16(status).unwrap(), Json(body)).into_response()
    }
}
```

---

## **CI/CD Pipeline**

### **GitHub Actions 流程**

```yaml
# .github/workflows/ci.yml
触发条件：PR 到 main / develop

Jobs（并行执行）：

backend-ci:
  - cargo fmt --check
  - cargo clippy -- -D warnings
  - cargo test --all
  - cargo nextest run --coverage
  - 覆盖率报告上传（codecov）
  - 覆盖率 < 80% → 构建失败

frontend-ci:
  - pnpm lint
  - pnpm type-check
  - pnpm test（Vitest）
  - pnpm build（Next.js 构建验证）

docker-build:
  依赖：backend-ci + frontend-ci 通过后
  - docker build backend
  - docker build frontend
  - 推送到 Registry（仅 main 分支）
```

### **部署流程**

```text
feature/* → PR → CI 通过 → Code Review → 合并到 develop
develop   → 自动部署到 Staging → 手动验收
main      → 自动部署到 Production（需 2 人 approve）
```

### **回滚策略**

```text
数据库迁移：使用 sqlx 支持 down migration
服务回滚：K8S 保留最近 3 个版本镜像，一键回滚
紧急回滚时间目标：< 5 分钟
```

---

# **15. Token 优化架构**

---

## **为什么必须在 V1 就解决**

Agent 是持久化的，会不断调用 LLM：复盘、记忆注入、任务推理。成本不控制，运营成本随 Agent 数量线性爆炸。

```text
不优化：
  1 个 Agent × 10 任务/天 × 4000 token/次 = 40,000 tokens/天
  100 个 Agent = 4,000,000 tokens/天 ≈ $12/天（gpt-4o pricing）

优化后（三层缓存 + 压缩）：
  实际 token 消耗下降 70-90%
  100 个 Agent ≈ $1.2-3.6/天
```

---

## **三层缓存架构**

```text
LLM 调用请求
      │
      ▼
┌─────────────────────────────────────────┐
│  Layer 1: Semantic Response Cache       │  命中率约 30-40%，延迟 <5ms
│  (pgvector，已有基础设施，零额外成本)    │
└────────────────────┬────────────────────┘
                     │ Miss
                     ▼
┌─────────────────────────────────────────┐
│  Layer 2: Prefix Cache（Provider 侧）   │  命中率约 60-90%（静态前缀）
│  Anthropic cache_control / OpenAI 自动  │  成本降至原价 10%
└────────────────────┬────────────────────┘
                     │ Miss（动态部分仍需计费）
                     ▼
┌─────────────────────────────────────────┐
│  Layer 3: LLMLingua-2 压缩注入          │  动态 Memory context 压缩 4-20x
│  （仅对 Memory 注入部分，不压缩系统 prompt）│
└────────────────────┬────────────────────┘
                     │
                     ▼
            实际 LLM API 调用
```

---

## **Layer 1：Semantic Response Cache**

### **原理**

把 Prompt 向量化，在 pgvector 中检索历史相似请求的 LLM 返回。若余弦相似度 ≥ 阈值，直接返回缓存结果，跳过 LLM 调用。

> **HARP 天然优势**：pgvector 已存在（Memory 表），直接复用同一个 PG 实例，新建 `llm_response_cache` 表即可，零新增基础设施成本。

### **数据结构**

```rust
pub struct LlmResponseCache {
    pub id: Uuid,
    pub cache_key: String,            // SHA256(prompt_template + agent_role)
    pub prompt_embedding: Vec<f32>,   // 1536 dims，用于语义相似检索
    pub prompt_hash: String,          // 精确匹配快速路径
    pub response: String,             // LLM 原始输出（JSON）
    pub model: String,                // "gpt-4o-mini" / "claude-3-5-haiku"
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub hit_count: i32,               // 被命中次数（用于分析）
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,    // Reflection: 24h；SOP推理: 7d
}
```

```sql
CREATE TABLE llm_response_cache (
  id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  cache_key        VARCHAR(64) NOT NULL,
  prompt_embedding vector(1536) NOT NULL,
  prompt_hash      VARCHAR(64) NOT NULL,
  response         TEXT NOT NULL,
  model            VARCHAR(100) NOT NULL,
  prompt_tokens    INTEGER NOT NULL DEFAULT 0,
  completion_tokens INTEGER NOT NULL DEFAULT 0,
  hit_count        INTEGER NOT NULL DEFAULT 0,
  created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at       TIMESTAMPTZ NOT NULL
);

-- 精确匹配（O(1)）
CREATE UNIQUE INDEX ON llm_response_cache (prompt_hash);
-- 语义相似检索
CREATE INDEX ON llm_response_cache
  USING ivfflat (prompt_embedding vector_cosine_ops);
-- 过期自动清理（配合 pg_cron）
CREATE INDEX ON llm_response_cache (expires_at);
```

### **命中策略**

```rust
pub struct SemanticCacheConfig {
    pub exact_hit_threshold: f32,    // 1.0  → 精确命中，直接返回
    pub semantic_hit_threshold: f32, // 0.92 → 高度相似，返回缓存
    pub soft_hit_threshold: f32,     // 0.85 → 返回缓存但标记 soft_hit，
                                     //         异步触发重新生成并更新缓存
}
```

### **各场景 TTL 与阈值**

| **LLM 调用场景** | **Semantic 阈值** | **TTL** | **预期命中率** |
|---|---|---|---|
| Reflection（同类型任务）| 0.92 | 24 小时 | ~35% |
| SOP 条件推理 | 0.90 | 7 天 | ~50% |
| Memory 一致性验证 | 0.88 | 12 小时 | ~40% |
| Growth 质量评分 | 0.90 | 48 小时 | ~45% |

---

## **Layer 2：Prefix Cache（Provider 侧）**

### **原理**

把 Prompt 中**不变的部分**放在前面，并打 `cache_control` 标记。Provider 缓存这段 prefix，后续调用若 prefix 相同，只对新增部分计费（Anthropic 计费为原价 10%，OpenAI 自动缓存免费）。

```text
Anthropic prefix cache TTL：5 分钟
缓存读取成本：$0.30/M tokens（原价 $3.00/M）
节省：90%
```

### **HARP Prompt 结构设计**

Agent 每次调用 LLM 的 Prompt 按 "静态 → 动态" 排列，静态部分打 cache breakpoint：

```text
┌──────────────────────────────────────────────┐ ← cache_control breakpoint
│  [STATIC] 系统人格 + 角色 + 核心指令           │   每次调用都相同，必缓存
│  例：你是 Agent {name}，角色是 {role}          │
│  你的核心职责是...（1200 tokens）              │
├──────────────────────────────────────────────┤ ← cache_control breakpoint
│  [STATIC] 可用 Skill 列表（Tool definitions） │   Skill 变化频率低，缓存 TTL=5min
│  [{skill_name, description, parameters}...]   │   平均 500-800 tokens
├──────────────────────────────────────────────┤ ← cache_control breakpoint
│  [QUASI-STATIC] SOP 上下文                    │   SOP 变化频率更低，缓存效果最好
│  当前执行的 SOP 节点定义和条件                 │   平均 300-600 tokens
└──────────────────────────────────────────────┘
  ↓ 以下为动态部分（不缓存，每次计费）
┌──────────────────────────────────────────────┐
│  [DYNAMIC] 当前任务描述 + 执行上下文           │   每次不同，约 200-400 tokens
│  [DYNAMIC] 注入的 Memory 片段                  │   压缩后注入，约 300-800 tokens
│  [DYNAMIC] 最近 N 条任务日志摘要               │   约 100-300 tokens
└──────────────────────────────────────────────┘
```

### **Rust 实现（rig 适配）**

```rust
// 构建带 cache breakpoint 的 Prompt
pub fn build_reflection_prompt(
    agent: &Agent,
    skills: &[Skill],
    sop_context: Option<&SopContext>,
    task: &Task,
    memory_chunks: &[MemoryChunk],
) -> Vec<Message> {
    let mut messages = vec![
        // Static prefix — 命中率最高
        Message::system_with_cache(
            format!("你是 Agent {}，角色是 {}。\n{}", 
                agent.name, agent.role, SYSTEM_CORE_INSTRUCTION),
        ),
        // Skill 列表 — 变化低，缓存效果好
        Message::system_with_cache(
            format!("可用 Skill：\n{}", skills_to_prompt(skills)),
        ),
    ];

    // SOP 上下文（如有）
    if let Some(sop) = sop_context {
        messages.push(Message::system_with_cache(
            format!("当前 SOP 上下文：\n{}", sop_to_prompt(sop)),
        ));
    }

    // 动态部分 — 不缓存，但已压缩
    messages.push(Message::user(
        format!("任务：{}\n\n相关记忆：\n{}\n\n执行日志摘要：\n{}",
            task_to_prompt(task),
            memory_chunks_to_prompt(memory_chunks),  // 已经过 LLMLingua 压缩
            task.log_summary(),
        )
    ));

    messages
}

// 带 cache_control 的消息构建（Anthropic）
impl Message {
    pub fn system_with_cache(content: String) -> Self {
        Self {
            role: "system".into(),
            content,
            cache_control: Some(CacheControl { cache_type: "ephemeral".into() }),
        }
    }
}
```

---

## **Layer 3：LLMLingua-2 Prompt 压缩**

### **原理**

Microsoft LLMLingua-2 是一个小型分类器，识别 Prompt 中信息熵低的 token 并删除，保留语义完整性。用于压缩注入 LLM 的 Memory 片段。

```text
RAG 检索到 5 条 Memory，原始合计 2000 tokens
经 LLMLingua-2 压缩（ratio=0.5）→ 约 1000 tokens
实测 RAG 性能：压缩后不降反升 +21.4%（信息密度更高）
生产案例：$42,000/月 → $2,100/月（一个 RAG 服务团队）
```

> **注意**：只对 Memory 注入片段压缩，**不压缩**系统 prompt 和 Tool 定义（会破坏结构）。

### **HARP 集成方案**

LLMLingua-2 是 Python 库。HARP 通过以下方式集成：

```text
方案 A（推荐）：独立 Python 微服务
  harp-compressor-service（FastAPI + LLMLingua-2）
  POST /compress { text, ratio, protect_tokens }
  → Rust 通过 HTTP 调用，结果缓存 24h（相同 Memory 内容不重复压缩）

方案 B：预处理阶段压缩
  Memory 写入时（swiftide pipeline 末端）同步压缩
  存储 compressed_content 字段（alongside 原文）
  检索时直接返回已压缩内容，零运行时开销
```

**推荐方案 B**（零运行时延迟，符合 HARP Rust 主栈）：

```sql
-- memories 表新增字段
ALTER TABLE memories ADD COLUMN compressed_content TEXT;
ALTER TABLE memories ADD COLUMN original_tokens INTEGER;
ALTER TABLE memories ADD COLUMN compressed_tokens INTEGER;
ALTER TABLE memories ADD COLUMN compression_ratio NUMERIC(4,2);
```

```rust
// swiftide 写入 pipeline 末端追加压缩步骤
pub async fn write_memory_with_compression(
    content: &str,
    compressor: &dyn TextCompressor,
) -> Result<Memory, AppError> {
    let compressed = compressor.compress(content, CompressionConfig {
        target_ratio: 0.5,       // 压缩到 50%
        protect_patterns: vec![  // 不压缩的关键词
            r"\b(失败|错误|异常|原因|改进)\b".into(),  // 复盘关键词
        ],
        min_tokens: 50,          // 短于 50 token 不压缩，性价比低
    }).await?;

    Ok(Memory {
        content: content.to_string(),
        compressed_content: Some(compressed.text),
        original_tokens: Some(compressed.original_tokens),
        compressed_tokens: Some(compressed.compressed_tokens),
        compression_ratio: Some(compressed.ratio),
        ..
    })
}
```

---

## **Memory 注入预算控制**

每次 LLM 调用，注入的 Memory 总量受硬性 token 预算约束：

```rust
pub struct MemoryInjectionBudget {
    pub total_budget: u32,       // 硬上限：2000 tokens
    pub working_alloc: u32,      // Working Memory：全量注入，约 200 tokens
    pub short_alloc: u32,        // Short Memory：top-3，约 400 tokens（压缩后）
    pub long_alloc: u32,         // Long Memory：top-2，约 800 tokens（压缩后）
    pub org_alloc: u32,          // Org Memory：top-1，约 600 tokens（压缩后）
}

// 注入选择策略
pub async fn select_memory_for_injection(
    agent_id: Uuid,
    task_context: &str,
    budget: &MemoryInjectionBudget,
) -> Vec<MemoryChunk> {
    // 1. Working Memory — 全量（当前任务直接相关，不过滤）
    let working = fetch_working_memory(agent_id).await;

    // 2. 其他层 — 语义检索，按 score 降序，超预算截断
    let short = semantic_search(agent_id, task_context, MemoryType::Short, 
        top_k=3, min_score=0.75).await;
    let long = semantic_search(agent_id, task_context, MemoryType::Long, 
        top_k=2, min_score=0.80).await;
    let org = semantic_search(agent_id, task_context, MemoryType::Organization, 
        top_k=1, min_score=0.85).await;

    // 3. 使用 compressed_content（已在写入时压缩）
    [working, short, long, org]
        .into_iter()
        .flatten()
        .take_within_budget(budget.total_budget)
        .collect()
}
```

### **Memory 自动摘要（防止记忆膨胀）**

```text
触发条件（定时任务，每日 03:00 UTC）：
  Short Memory > 30 条 → 合并最早 20 条为 1 条摘要，写入 Long Memory
  Long Memory > 100 条 → 按 topic 聚类，每个 cluster 摘要为 1 条，删除原文
```

```rust
// Growth Engine 周期任务
pub async fn auto_compress_memories(agent_id: Uuid) -> Result<(), AppError> {
    let short_count = count_memory(agent_id, MemoryType::Short).await?;
    if short_count > 30 {
        let oldest_20 = fetch_oldest_memories(agent_id, MemoryType::Short, 20).await?;
        let summary = llm_summarize(oldest_20).await?;
        write_memory(Memory {
            memory_type: MemoryType::Long,
            content: summary,
            source: MemorySource::AutoCompress,
            ..
        }).await?;
        delete_memories(oldest_20.iter().map(|m| m.id).collect()).await?;
    }
    Ok(())
}
```

---

## **成本监控**

```rust
// 每次 LLM 调用后记录到 llm_cost_logs 表
pub struct LlmCostLog {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub call_type: String,         // "reflection" | "sop_reasoning" | "growth_score"
    pub model: String,
    pub cache_layer: CacheLayer,   // Semantic / Prefix / None
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub cached_tokens: i32,        // Prefix Cache 命中的 tokens
    pub cost_usd: f64,             // 实际扣费
    pub saved_usd: f64,            // 本次节省（与无缓存对比）
    pub created_at: DateTime<Utc>,
}

pub enum CacheLayer {
    SemanticHit,   // Layer 1 命中，cost = 0
    PrefixHit,     // Layer 2 命中（部分），cost = 10% of cached tokens
    NoCache,       // 全量调用
}
```

### **监控指标（暴露给 Prometheus）**

```text
harp_llm_semantic_cache_hit_rate   # 目标 ≥ 30%
harp_llm_prefix_cache_hit_rate     # 目标 ≥ 60%（静态前缀占比）
harp_llm_tokens_per_reflection     # 目标 ≤ 1500 tokens（压缩后）
harp_llm_cost_usd_per_agent_day    # 告警阈值：> $0.10/agent/天
harp_memory_compression_ratio      # 目标 0.45-0.55（压缩 45-55%）
```

---

## **整体节省预估（100 Agent，10 任务/天）**

| **优化层** | **机制** | **节省比例** | **覆盖场景** |
|---|---|---|---|
| Semantic Response Cache | pgvector 相似命中 | ~35% 调用跳过 | Reflection、SOP 推理 |
| Prefix Cache | 静态前缀 10% 计费 | 已调用部分节省 ~55% | 所有 LLM 调用 |
| LLMLingua-2 压缩 | Memory 注入压缩 50% | 动态部分节省 ~50% | Memory 注入 |
| Memory 注入预算 | 硬性 2000 token 上限 | 防止 token 爆炸 | 所有场景 |
| **合计** | 三层叠加 | **节省 75-90%** | **全链路** |

---

# **16. 源码参考索引**

> 开发时直接打开对应文件，不要凭记忆猜 API。
> 所有仓库已克隆到 `~/Projects/harp/research/`

---

## **Memory 相关**

| **任务** | **参考文件** | **关键符号** |
|---|---|---|
| Memory RAG 写入 pipeline | `swiftide/examples/index_md_into_pgvector.rs` | `indexing::Pipeline`, `PgVector::builder()` |
| Memory RAG 检索 pipeline | `swiftide/examples/query_pipeline.rs` | `query::Pipeline`, `GenerateSubquestions`, `Summary` |
| pgvector 表结构 | `swiftide/swiftide-integrations/src/pgvector/persist.rs` | `PgVector`, `EmbeddedField` |
| Token 预算 / 滑窗内存 | `rig/crates/rig-memory/examples/agent_with_memory_policies.rs` | `TokenWindowMemory`, `SlidingWindowMemory` |
| pgvector 余弦检索 | `rig/crates/rig-postgres/src/lib.rs` | `PostgresVectorStore`, `PgVectorDistanceFunction::Cosine` |
| Memory 内容安全校验 | `garudust-agent/crates/garudust-core/src/memory.rs` | `validate_memory_content`, `MAX_ENTRY_CHARS=500`, XML 注入防御 |
| Memory 分类体系参考 | `garudust-agent/crates/garudust-core/src/memory.rs` | `MemoryCategory::{Fact,Preference,Skill,Project,Other}` |

## **Skill 相关**

| **任务** | **参考文件** | **关键符号** |
|---|---|---|
| SKILL.md 格式解析 | `garudust-agent/crates/garudust-tools/src/toolsets/skills.rs` | `parse_skill_md`, `Skill` struct |
| Skill 目录加载 | `garudust-agent/crates/garudust-tools/src/toolsets/skills.rs` | `load_skills_from_dir`, `build_skills_index` |
| Skill 写入工具（LLM 自建 Skill）| `garudust-agent/crates/garudust-tools/src/toolsets/skills.rs` | `WriteSkill::execute` |
| Skill 权限合并（deny-wins）| `garudust-agent/crates/garudust-core/src/memory.rs` | `SkillPermissions::merge` |
| Tool 注册 (#[tool] 宏) | `AutoAgents/crates/autoagents-derive/` | `#[tool]`, `#[agent]`, `ToolRuntime` trait |

## **Session / 持久化 相关**

| **任务** | **参考文件** | **关键符号** |
|---|---|---|
| PostgreSQL session 持久化 | `adk-rust/adk-session/src/postgres.rs` | `PostgresSessionService`, advisory lock, JSONB state |
| Session migration | `adk-rust/adk-session/src/postgres.rs` | `PG_SESSION_MIGRATIONS`, `ADVISORY_LOCK_KEY` |
| Session 事件 | `adk-rust/adk-session/src/event.rs` | `Event`, `Events`, `AppendEventRequest` |

## **Agent 执行相关**

| **任务** | **参考文件** | **关键符号** |
|---|---|---|
| ReAct executor 用法 | `AutoAgents/examples/basic/` | `ReActAgent`, `AgentBuilder`, `SlidingWindowMemory` |
| 多 Agent pub/sub | `AutoAgents/examples/design_patterns/` | 参见 Routing / Parallel 示例 |
| rig agent builder | `rig/examples/agent.rs` | `client.agent().preamble().tool().build()` |
| rig 结构化输出 | `rig/crates/rig-core/src/extractor.rs` | `ExtractorAgent` |

## **Goal / 跨 Session 目标注入**

| **任务** | **参考文件** | **关键符号** |
|---|---|---|
| 会话级目标持久化（参考）| `garudust-agent/crates/garudust-memory/src/goal_store.rs` | `GoalStore::set/get/clear`（文件哈希方式）|

---

# **17. 开发阶段规划**

| **Epic** | **内容** | **预计周期** |
|---|---|---|
| Epic-0 基础设施 | AgentTeam 初始化、仓库、环境、设计系统、测试基础 | 1 周 |
| Epic-1 Runtime | Agent 实体、状态机、Runtime Kernel | 1.5 周 |
| Epic-2 Memory | swiftide 接入、pgvector 索引、权限隔离 | 1 周 |
| Epic-3 Skill | autoagents Tool 注册、Skill 执行、MCP 接入 | 1 周 |
| Epic-4 SOP | SOP Builder、DAG Runner（Prolog 条件分支）| 2 周 |
| Epic-5 Growth | 成长模型、等级体系、GrowthProfile | 1 周 |
| Epic-6 Reflection | rig 调用、复盘 Prompt、结构化输出、Memory 写入 | 0.5 周 |
| Epic-7 Event Bus | async-nats 接入、事件持久化 | 0.5 周 |
| Epic-8 HTML Runtime | Agent 主页、状态面板、Team 可视化 | 1.5 周 |
| Epic-9 集成验收 | 端到端压测、Bug 修复、安全扫描 | 1 周 |

**总周期：约 10 周（含缓冲）。**

节省 7 周的原因：
- `rig`：无需手写 LLM HTTP client + provider 适配（节省 ~2 周）
- `autoagents`：无需手写 ReAct 执行器 + Tool 注册框架（节省 ~2 周）
- `swiftide`：无需手写 embedding pipeline + 批量写入逻辑（节省 ~2 周）
- PrologAgentTeam 代码直接复制（节省 ~1 周）

MVP 最小闭环可裁剪至 Epic 0-3 + 6 + 7（约 5 周），先跑通"创建→执行→记忆→复盘"核心环路。
