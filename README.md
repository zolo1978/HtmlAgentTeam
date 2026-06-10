<p align="center">
  <img src="docs/banner.svg" alt="HARP — HTML Agent Runtime Platform" width="100%"/>
</p>

<p align="center">
  <a href="https://github.com/zolo1978/HtmlAgentTeam/actions"><img src="https://img.shields.io/github/actions/workflow/status/zolo1978/HtmlAgentTeam/ci.yml?branch=main&style=flat-square&label=CI&color=4f46e5" alt="CI"/></a>
  <img src="https://img.shields.io/badge/Rust-1.79+-fb923c?style=flat-square&logo=rust&logoColor=white" alt="Rust"/>
  <img src="https://img.shields.io/badge/Next.js-15-f472b6?style=flat-square&logo=next.js&logoColor=white" alt="Next.js"/>
  <img src="https://img.shields.io/badge/PostgreSQL-16+pgvector-38bdf8?style=flat-square&logo=postgresql&logoColor=white" alt="PostgreSQL"/>
  <img src="https://img.shields.io/badge/status-in%20development-fbbf24?style=flat-square" alt="Status"/>
  <img src="https://img.shields.io/badge/license-MIT-a78bfa?style=flat-square" alt="License"/>
</p>

<br/>

> **AI Agents are not tools — they are persistent software entities with identity, memory, and growth.**

HARP is an open-source **persistent Agent runtime platform**. Every Agent lives in its own HTML home, accumulates memories across sessions, reflects on its work, levels up over time, and executes complex workflows through a typed SOP engine — all backed by a three-layer token cache that cuts LLM costs by up to 90%.

---

## Why HARP?

Most Agent frameworks treat each run as stateless. HARP is built on the opposite premise:

| Conventional Agents | HARP Agents |
|---------------------|-------------|
| Stateless per call | Persistent across sessions |
| No memory between runs | 4-layer memory with vector retrieval |
| Fixed capability | L1–L6 growth system |
| No self-improvement | Structured reflection engine |
| Ad-hoc tool use | Typed SKILL.md + Prolog rule engine |
| Full LLM cost every time | 75–90% cost reduction via 3-layer cache |

---

## Core Concepts

### Agent Lifecycle

```
Created ──▶ Activated ──▶ Working ──▶ Reflecting ──▶ Evolving ──▶ Activated
                                                            │
                                               Failed ◀────┤ (recoverable)
                                              Archived ◀───┘ (terminal)
```

### Memory Architecture

```
┌─────────────────────────────────────────────────────┐
│  Working Memory      (current task context)          │
├─────────────────────────────────────────────────────┤
│  Short-Term Memory   (top-3, score ≥ 0.75)          │
├─────────────────────────────────────────────────────┤
│  Long-Term Memory    (top-2, score ≥ 0.80)          │
├─────────────────────────────────────────────────────┤
│  Organization Memory (top-1, score ≥ 0.85)          │
└─────────────────────────────────────────────────────┘
        Total injection budget: 2,000 tokens
```

### Token Cost Optimization

```
Request
  ├─▶ Semantic Cache   (pgvector cosine ≥ 0.92) ──▶ return cached response  ✦ free
  ├─▶ Prefix Cache     (Anthropic cache_control)  ──▶ $0.30/M vs $3.00/M   ✦ 90% off
  └─▶ LLM              (LLMLingua-2 compression)  ──▶ 4–20× token reduction
```

Estimated cost for 100 active agents: **$1.2–3.6 / day** (vs. ~$12/day without caching)

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| **Runtime** | Rust · Tokio · Axum 0.7 |
| **LLM Client** | [rig-core](https://github.com/0xPlaygrounds/rig) 0.38 · 20+ providers |
| **Memory Store** | PostgreSQL 16 · pgvector · swiftide 0.38 |
| **Agent Execution** | autoagents 0.32 (ReAct loop) |
| **Memory Injection** | rig-memory 0.38 · TokenWindowMemory |
| **Logic Engine** | scryer-prolog 0.9 (pure Rust) |
| **Event Bus** | NATS JetStream |
| **Frontend** | Next.js 15 · Zustand · shadcn/ui · Framer Motion |
| **Deployment** | Railway (MVP) → Fly.io → Kubernetes |

---

## Getting Started

**Prerequisites:** Rust 1.79+, Node.js 22+, Docker

```bash
# Clone
git clone https://github.com/zolo1978/HtmlAgentTeam.git && cd HtmlAgentTeam

# Configure environment
cp .env.example .env
# → Fill in OPENAI_API_KEY, ANTHROPIC_API_KEY, JWT_SECRET

# Start infrastructure (PostgreSQL + Redis + NATS)
cd infra && docker compose up -d && cd ..

# Run backend
cd backend && cargo run

# Run frontend (new terminal)
cd frontend && npm install && npm run dev
```

Open [http://localhost:3000](http://localhost:3000)

---

## Project Structure

```
.
├── backend/                  # Rust — Axum API server
│   ├── src/
│   │   ├── api/              # 36 REST endpoints
│   │   ├── agent/            # State machine + lifecycle
│   │   ├── auth/             # JWT (Access 15min / Refresh 7day)
│   │   ├── memory/           # 4-layer memory + vector retrieval
│   │   ├── skill/            # SKILL.md parser + executor
│   │   ├── growth/           # GrowthScore + level-up engine
│   │   ├── reflection/       # Structured reflection (ExtractorAgent)
│   │   ├── sop/              # DAG workflow runner
│   │   ├── prolog/           # scryer-prolog rule engine
│   │   ├── llm/              # 3-layer token cache
│   │   └── event/            # NATS JetStream publisher/consumer
│   └── migrations/           # PostgreSQL migrations (advisory lock)
├── frontend/                 # Next.js 15 — App Router
│   └── src/
│       ├── app/              # Pages
│       ├── components/       # shadcn/ui + custom
│       ├── store/            # Zustand slices
│       └── lib/              # API client + utilities
├── infra/
│   └── docker-compose.yml    # PG16+pgvector · Redis · NATS
├── docs/
│   ├── TDD.md                # Technical Design Document
│   ├── TECH_SPEC.md          # Full spec: DDL · WS · API · Prolog · Deploy
│   └── TASK_LIST.md          # 212-task implementation checklist
└── .github/workflows/ci.yml  # Rust test/clippy/fmt · TS type-check/lint
```

---

## Roadmap

| Sprint | Weeks | Goal |
|--------|-------|------|
| S0 | 1–2 | Infrastructure · Auth · Agent CRUD |
| S1 | 3–4 | Memory system · pgvector · token budget |
| S2 | 5 | 3-layer token cache · LLMLingua-2 |
| S3 | 6–7 | Task execution · ReAct · NATS events |
| S4 | 8 | Skill system · SKILL.md runtime |
| S5 | 9–10 | SOP DAG · Prolog rule engine |
| S6 | 11–12 | Reflection engine · Growth system |
| S7 | 13–14 | Frontend core pages |
| S8 | 15 | WebSocket · Growth UI · Skill tree |
| S9 | 16 | QA · Railway deploy · benchmarks |

Full task breakdown: [docs/TASK_LIST.md](docs/TASK_LIST.md)

---

## Documentation

| Document | Description |
|----------|-------------|
| [TDD.md](docs/TDD.md) | Architecture, module boundaries, source code references |
| [TECH_SPEC.md](docs/TECH_SPEC.md) | Complete DDL · WebSocket protocol · API schema · Prolog rules · Error codes · Deployment |
| [TASK_LIST.md](docs/TASK_LIST.md) | 212 actionable tasks with source paths, targets, and sprint assignments |

---

## Contributing

This project is in active development. Contributions, issues, and design feedback are welcome.

```bash
cargo test          # run backend tests
cargo clippy        # lint
cd frontend && npm run type-check
```

---

## License

MIT © 2026 HARP Contributors
