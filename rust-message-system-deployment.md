# Rust 多租户消息系统 - 部署计划

**Generated from**: `rust-message-system-solution.md`
**Created**: 2026-02-26
**Session**: `deploy-rust-msg-001`

---

## Progress Overview

| Phase | Tasks | Completed | In Progress | Blocked | Progress |
|-------|-------|-----------|-------------|---------|----------|
| P0 - 基础设施准备 | 7 | 0 | 0 | 0 | 0% |
| P1 - 核心服务实现 | 8 | 0 | 0 | 0 | 0% |
| P2 - 消息队列集成 | 4 | 0 | 0 | 0 | 0% |
| P3 - WebSocket 实时通信 | 4 | 0 | 0 | 0 | 0% |
| P4 - 多渠道推送集成 | 5 | 1 | 2 | 0 | 40% |
| P5 - 测试与优化 | 5 | 0 | 0 | 0 | 0% |
| P6 - 部署与监控 | 6 | 0 | 0 | 0 | 0% |
| **TOTAL** | **39** | **0** | **0** | **0** | **0%** |

---

## Phase 0: 基础设施准备

**Priority**: CRITICAL
**Estimated Duration**: 2-3 days
**Dependencies**: None

| ID | Task | Status | Owner | Notes |
|----|------|--------|-------|-------|
| P0-T001 | PostgreSQL 数据库部署与配置 | Not Started | - | Docker 部署，创建 message_system 数据库 |
| P0-T002 | Redis 缓存服务部署 | Not Started | - | Docker 部署，配置持久化 |
| P0-T003 | RabbitMQ 消息队列部署 | Not Started | - | 启用管理界面，创建 message_queue |
| P0-T004 | 数据库迁移文件创建 | Not Started | - | 执行 migrations 目录下所有 SQL |
| P0-T005 | 环境变量配置文件创建 | Not Started | - | .env 文件配置（数据库、Redis、JWT等） |
| P0-T006 | Rust 项目初始化 | Not Started | - | cargo new，配置 Cargo.toml 依赖 |
| P0-T007 | 基础目录结构创建 | Not Started | - | models/handlers/services/repositories 等 |

---

## Phase 1: 核心服务实现

**Priority**: HIGH
**Estimated Duration**: 5-7 days
**Dependencies**: P0 completed

| ID | Task | Status | Owner | Notes |
|----|------|--------|-------|-------|
| P1-T001 | 实现错误处理模块 (error.rs) | Not Started | - | AppError 枚举，IntoResponse trait |
| P1-T002 | 实现配置管理模块 (config.rs) | Not Started | - | Config 结构体，环境变量加载 |
| P1-T003 | 实现数据模型 (models/) | Not Started | - | Message, User, Organization, Template 等 |
| P1-T004 | 实现 Repository 层 (repositories/) | Not Started | - | MessageRepository, UserRepository, OrganizationRepository |
| P1-T005 | 实现 MessageService 核心逻辑 | Not Started | - | send_message, render_template 方法 |
| P1-T006 | 实现 TargetResolver 目标解析器 | Not Started | - | resolve_target_users，支持 user/org/role/custom |
| P1-T007 | 实现 TemplateService 模板服务 | Not Started | - | 模板 CRUD，变量渲染 |
| P1-T008 | 实现 API Handler (handlers/) | Not Started | - | message_handler, template_handler, admin_handler |

---

## Phase 2: 消息队列集成

**Priority**: HIGH
**Estimated Duration**: 3-4 days
**Dependencies**: P1 completed

| ID | Task | Status | Owner | Notes |
|----|------|--------|-------|-------|
| P2-T001 | 实现 RabbitMQ Producer (queue/producer.rs) | Not Started | - | publish_message, publish_message_with_delay |
| P2-T002 | 实现 RabbitMQ Consumer (queue/consumer.rs) | Not Started | - | 消息消费，ACK/NACK 处理 |
| P2-T003 | 实现消息处理流程 | Not Started | - | process_message 函数，调用 TargetResolver + PushService |
| P2-T004 | 集成消息队列到主服务 | Not Started | - | tokio::spawn 启动消费者 |

---

## Phase 3: WebSocket 实时通信

**Priority**: HIGH
**Estimated Duration**: 3-4 days
**Dependencies**: P1, P2 completed

| ID | Task | Status | Owner | Notes |
|----|------|--------|-------|-------|
| P3-T001 | 实现 WebSocket Handler (websocket/handler.rs) | Not Started | - | ws_handler, 连接管理，心跳处理 |
| P3-T002 | 实现 WebSocket Manager (websocket/manager.rs) | Not Started | - | 连接存储，send_to_user, broadcast_to_tenant |
| P3-T003 | 实现 Redis 在线状态管理 (cache/redis_client.rs) | Not Started | - | set_user_online, is_user_online |
| P3-T004 | 集成 WebSocket 到 Axum 路由 | Not Started | - | /ws/:tenant_id 端点 |

---

## Phase 4: 多渠道推送集成

**Priority**: MEDIUM
**Estimated Duration**: 4-5 days
**Dependencies**: P3 completed

| ID | Task | Status | Owner | Notes |
|----|------|--------|-------|-------|
| P4-T001 | 实现 PushService 推送服务 | Completed | - | push_to_users, 在线/离线处理逻辑 |
| P4-T002 | 实现邮件推送 (Lettre) | In Progress | - | 邮件渠道插件已创建，待接入 SMTP 详情 |
| P4-T003 | 实现钉钉机器人推送 | In Progress | - | 钉钉渠道插件已创建，待接入 webhook 详情 |
| P4-T004 | 实现用户消息设置管理 | Not Started | - | user_message_settings CRUD |
| P4-T005 | 实现推送日志记录 (message_push_logs) | Completed | - | log_push 已在 PushService 中使用 |

---

## Phase 5: 测试与优化

**Priority**: MEDIUM
**Estimated Duration**: 4-5 days
**Dependencies**: P4 completed

| ID | Task | Status | Owner | Notes |
|----|------|--------|-------|-------|
| P5-T001 | 编写单元测试 | Not Started | - | Repository 层，Service 层测试 |
| P5-T002 | 编写集成测试 | Not Started | - | API 端到端测试 |
| P5-T003 | 性能压力测试 | Not Started | - | 并发消息发送，WebSocket 连接数测试 |
| P5-T004 | 数据库查询优化 | Not Started | - | EXPLAIN ANALYZE，索引优化 |
| P5-T005 | Redis 缓存策略优化 | Not Started | - | 过期时间设置，缓存预热 |

---

## Phase 6: 部署与监控

**Priority**: HIGH
**Estimated Duration**: 2-3 days
**Dependencies**: P5 completed

| ID | Task | Status | Owner | Notes |
|----|------|--------|-------|-------|
| P6-T001 | Docker 镜像构建 | Not Started | - | 多阶段构建，镜像优化 |
| P6-T002 | Docker Compose 编排 | Not Started | - | 服务编排，依赖管理 |
| P6-T003 | 生产环境配置 | Not Started | - | .env.production，TLS 配置 |
| P6-T004 | 日志聚合配置 (tracing) | Not Started | - | JSON 格式输出，日志级别 |
| P6-T005 | 监控集成 (Prometheus + Grafana) | Not Started | - | 指标暴露，仪表盘配置 |
| P6-T006 | 健康检查端点实现 | Not Started | - | /health 端点，依赖检查 |

---

## Session Continuity

**Session ID**: `deploy-rust-msg-001`
**Last Updated**: 2026-02-26

### Next Steps (当恢复会话时执行)

1. 执行 `P0-T001` - 部署 PostgreSQL
2. 更新任务状态为 "In Progress"
3. 完成后更新为 "Completed"，继续下一个任务

### Blocked Issues

*No blocked issues recorded yet.*

### Deferred Tasks

*No deferred tasks recorded yet.*

---

## Appendix: 快速启动命令

```bash
# Phase 0: 基础设施
docker run -d --name postgres -e POSTGRES_PASSWORD=password -e POSTGRES_DB=message_system -p 5432:5432 postgres:15

docker run -d --name redis -p 6379:6379 redis:7

docker run -d --name rabbitmq -p 5672:5672 -p 15672:15672 rabbitmq:3-management

# 数据库迁移
sqlx migrate run

# 开发运行
cargo run

# 生产构建
cargo build --release
```
