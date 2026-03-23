# Rust 多租户系统消息解决方案

## 项目概述

基于 Rust 构建的高性能、多租户、多组织的系统消息推送平台，采用现代化的 Web 技术栈和最佳实践。

## 技术栈

- **Web 框架**: Axum (高性能异步 Web 框架)
- **数据库**: PostgreSQL + SQLx (编译时 SQL 检查)
- **缓存**: Redis (tokio-redis)
- **消息队列**: RabbitMQ (lapin) / Redis Streams
- **WebSocket**: Axum WebSocket + tokio-tungstenite
- **序列化**: Serde (JSON)
- **异步运行时**: Tokio
- **日志**: tracing + tracing-subscriber
- **配置管理**: config + dotenv
- **ORM 替代**: SeaORM / SQLx (本方案使用 SQLx)
- **认证**: jsonwebtoken (JWT)
- **任务调度**: tokio-cron-scheduler

## 项目结构

```
message-system/
├── Cargo.toml
├── .env
├── migrations/              # 数据库迁移文件
│   ├── 20240101_create_tenants.sql
│   └── ...
├── src/
│   ├── main.rs             # 入口文件
│   ├── config.rs           # 配置管理
│   ├── error.rs            # 统一错误处理
│   ├── models/             # 数据模型
│   │   ├── mod.rs
│   │   ├── tenant.rs
│   │   ├── message.rs
│   │   └── message_template.rs
│   ├── handlers/           # API 处理器
│   │   ├── mod.rs
│   │   ├── message_handler.rs
│   │   ├── template_handler.rs
│   │   ├── setting_handler.rs
│   │   └── admin_handler.rs
│   ├── services/           # 业务逻辑层
│   │   ├── mod.rs
│   │   ├── message_service.rs
│   │   ├── push_service.rs
│   │   ├── target_resolver.rs
│   │   └── template_service.rs
│   ├── repositories/       # 数据访问层
│   │   ├── mod.rs
│   │   └── message_repository.rs
│   ├── websocket/          # WebSocket 处理
│   │   ├── mod.rs
│   │   ├── handler.rs
│   │   └── manager.rs
│   ├── queue/              # 消息队列
│   │   ├── mod.rs
│   │   ├── producer.rs
│   │   └── consumer.rs
│   ├── cache/              # 缓存层
│   │   ├── mod.rs
│   │   └── redis_client.rs
│   ├── middleware/         # 中间件
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   └── tenant.rs
│   └── utils/              # 工具函数
│       ├── mod.rs
│       ├── jwt.rs
│       └── pagination.rs
└── tests/
    └── integration/
```

---

## 系统架构设计

### 实时通信与消息队列的架构关系

本系统采用 **WebSocket + RabbitMQ** 的混合架构，实现高可靠、高并发的消息推送。

#### 1. 架构概览

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              客户端 (浏览器/App)                          │
│  ┌─────────────┐                                                        │
│  │ WebSocket连接 │ ←───────────────────────────────────────────────────┐ │
│  └──────┬──────┘                                                     │ │
└─────────┼────────────────────────────────────────────────────────────┘ │
          │                                                              │
          │ 1. 建立长连接                                                  │
          │ 2. 实时推送/接收消息                                             │
          │                                                              │
┌─────────▼────────────────────────────────────────────────────────────┐ │
│                          API Gateway (Axum)                          │ │
│  ┌─────────────────┐    ┌──────────────────────────────────────────┐  │ │
│  │  WS /ws/:tenant │    │  HTTP POST /api/messages/send            │  │ │
│  │     ↑↓ 双向     │    │     ↓                                    │  │ │
│  └─────┬───────────┘    └─────┬────────────────────────────────────┘  │ │
└────────┼───────────────────────┼───────────────────────────────────────┘
         │                       │
         │                       │ 3. 调用发送接口
         │                       │
┌────────▼───────────────────────▼─────────────────────────────────────┐
│                          MessageService                              │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │  send_message()                                                  │ │
│  │    ├── 创建消息记录 (DB)                                          │ │
│  │    ├── 保存目标规则 (DB)                                          │ │
│  │    └── 发布到消息队列 → publish_message(message_id)                │ │
│  └─────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────┬──────────────────────────────────────────┘
                              │ 4. 异步投递
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│                        RabbitMQ 消息队列                              │
│  ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐  │
│  │  message_queue  │───→│ message_queue    │───→│ message_queue   │  │
│  │    (主队列)      │    │    _retry (重试)  │    │    _dlq (死信)   │  │
│  └─────────────────┘    └──────────────────┘    └─────────────────┘  │
└─────────────────────────────┬──────────────────────────────────────────┘
                              │ 5. 消费消息
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│                        Queue Consumer                                │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │  process_message(message_id)                                     │ │
│  │    ├── 获取消息和目标用户                                         │ │
│  │    ├── 解析目标用户列表                                           │ │
│  │    └── 调用 PushService.push_to_users()                          │ │
│  └─────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────┬──────────────────────────────────────────┘
                              │ 6. 多渠道推送
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│                        PushService                                   │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐   │
│  │  WebSocketChannel │  │  EmailChannel    │  │  DingTalkChannel    │   │
│  │    (实时通道)      │  │    (异步邮件)     │  │    (企业IM)         │   │
│  │                   │  │                   │  │                     │   │
│  │  ws_manager.send  │  │  SMTP发送         │  │  Webhook调用         │   │
│  │     ↓             │  │     ↓             │  │     ↓               │   │
│  │  在线用户←───────────┘  │  离线用户收到邮件  │  │  移动端收到通知      │   │
│  └─────────────────┘  └─────────────────┘  └─────────────────────┘   │
└──────────────────────────────────────────────────────────────────────┘
                              │
                              │ 7. WebSocket 实时推送
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│                      WebSocketManager                                │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │  connections: HashMap<(tenant_id, user_id), UnboundedSender>    │ │
│  │                                                                 │ │
│  │  send_to_user(tenant_id, user_id, payload)                      │ │
│  │    └── 找到对应连接 → tx.send(msg) → 推送到客户端                   │ │
│  └─────────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────┘
```

#### 2. 分层职责

| 层级 | 组件 | 职责 |
|------|------|------|
| **接入层** | WebSocket | 维护客户端长连接，双向实时通信 |
| **业务层** | MessageService | 处理发送请求，持久化消息 |
| **队列层** | RabbitMQ | 异步解耦，削峰填谷，保证可靠投递 |
| **推送层** | PushService | 多渠道并行推送 |
| **管理器** | WebSocketManager | 管理连接状态，路由消息到在线用户 |

#### 3. 核心流程

```
发送消息 → 存DB → 入队(MQ) → 消费 → 解析目标用户 → 并行推送各渠道
                                   ↓
                              ┌────────┐
                              │WebSocket│ → 在线用户立即收到 (实时)
                              │ Email   │ → 离线/备用渠道 (可靠)
                              │DingTalk │ → 移动端推送 (可达)
                              └────────┘
```

#### 4. 设计决策说明

| 设计决策 | 原因 |
|---------|------|
| **MQ 在 WebSocket 之前** | 发送是异步的，即使推送失败也可重试；削峰保护 WebSocket 服务 |
| **WebSocket 作为渠道之一** | 只是众多推送渠道中的一个，统一抽象为 `MessageChannel` |
| **消费端再查DB** | 消息队列只存ID，消费时查最新状态，避免消息过期/重复问题 |
| **DLQ 死信队列** | 最终失败的消息不丢失，可人工处理或后续补偿 |
| **重试机制** | 最大3次重试，指数退避，避免瞬时故障导致消息丢失 |

#### 5. 时序图

```
Client          API             MQ            Consumer        PushService      WSManager
  │              │              │               │                │               │
  │ ──────────WS连接──────────→ │               │                │               │
  │              │              │               │                │               │
  │ ─发送消息───→│              │               │                │               │
  │              │ ──存DB────→ │               │                │               │
  │              │ ──发布MQ────→│               │                │               │
  │ ←─返回ID────│              │               │                │               │
  │              │              │               │                │               │
  │              │              │ ──消费消息───→│                │               │
  │              │              │               │ ──解析用户────→│               │
  │              │              │               │                │ ──遍历渠道───→│
  │              │              │               │                │               │
  │ ←──────────实时推送───────────────WebSocketChannel.send()───────────────────→│
  │              │              │               │                │               │
```

---

## 消息源与消息目标数据流架构

### 架构概览

本消息系统支持多种**消息源**向多种**目标类型**分发消息，通过统一的渠道抽象实现灵活的推送策略。

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                              消息源 (Message Source)                                 │
├─────────────────┬─────────────────┬─────────────────┬───────────────────────────────┤
│   系统消息源     │   组织消息源     │   审批任务源    │      外部集成源                │
│   (System)      │   (Org)         │   (Workflow)    │      (External)               │
├─────────────────┼─────────────────┼─────────────────┼───────────────────────────────┤
│ • 系统公告      │ • 部门通知      │ • 待办任务      │ • 第三方系统 webhook          │
│ • 安全提醒      │ • 组织变更      │ • 审批结果      │ • 定时任务触发                │
│ • 维护通知      │ • 活动通知      │ • 抄送消息      │ • 数据同步事件                │
└────────┬────────┴────────┬────────┴────────┬────────┴────────┬──────────────────────┘
         │                 │                 │                 │
         └─────────────────┴────────┬────────┴─────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                          消息分发服务 (Message Dispatcher)                           │
│  ┌───────────────────────────────────────────────────────────────────────────────┐  │
│  │  dispatch(message, source_type, target_rules)                                  │  │
│  │    ├── 根据 source_type 设置消息类别                                            │  │
│  │    ├── 根据 target_rules 解析目标用户列表                                       │  │
│  │    └── 调用各渠道推送器并行发送                                                 │  │
│  └───────────────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────┬───────────────────────────────────────────────────┘
                                  │
          ┌───────────────────────┼───────────────────────┐
          │                       │                       │
          ▼                       ▼                       ▼
┌─────────────────┐   ┌─────────────────┐   ┌─────────────────────────────────────────┐
│   目标类型解析    │   │   渠道选择策略   │   │           推送渠道实现                   │
├─────────────────┤   ├─────────────────┤   ├─────────────┬─────────────┬─────────────┤
│ • 目标组织       │   │ • WebSocket     │   │  WebSocket  │   Email     │  DingTalk   │
│   (org_ids)     │   │ • Email         │   │   (实时)    │   (邮件)    │   (钉钉)    │
│ • 目标角色       │   │ • DingTalk      │   ├─────────────┼─────────────┼─────────────┤
│   (role_codes)  │   │ • LogFile       │   │   SMS       │   LogFile   │   (其他)    │
│ • 目标用户       │   │ • SMS           │   │   (短信)    │  (日志STUB) │             │
│   (user_ids)    │   │                 │   │             │             │             │
│ • 自定义条件     │   │                 │   │             │             │             │
└─────────────────┘   └─────────────────┘   └─────────────┴─────────────┴─────────────┘
```

### 消息源类型定义

| 消息源 | source_type | 说明 | 典型场景 |
|--------|-------------|------|----------|
| **系统** | `system` | 平台级消息，面向所有用户或特定范围 | 系统公告、安全提醒、版本更新 |
| **组织** | `organization` | 部门/组织级消息，面向组织内成员 | 部门通知、组织活动、人员变更 |
| **审批任务** | `workflow` | 流程审批相关消息，面向任务相关人 | 待办提醒、审批结果、抄送通知 |
| **外部集成** | `external` | 第三方系统触发的消息 | API调用、Webhook事件、数据同步 |

### 目标类型定义

| 目标类型 | target_type | 解析方式 | 数据来源 |
|----------|-------------|----------|----------|
| **指定用户** | `user` | 直接使用 user_ids | 请求传入（外部服务已解析） |
| **组织架构** | `org` | 外部服务解析 org_ids → user_ids | 外部组织服务 |
| **角色** | `role` | 外部服务解析 role_codes → user_ids | 外部权限服务 |
| **自定义** | `custom` | 外部服务根据条件解析 | 外部查询服务 |

### 消息类型映射

```
消息源 + 业务场景 → 消息类型 (msg_type)

系统 (system)
  ├── 公告通知 → system_announcement
  ├── 安全提醒 → system_security
  └── 维护通知 → system_maintenance

组织 (organization)
  ├── 部门通知 → org_department
  ├── 组织变更 → org_change
  └── 活动通知 → org_activity

审批任务 (workflow)
  ├── 待办任务 → workflow_todo
  ├── 审批结果 → workflow_result
  └── 抄送通知 → workflow_cc
```

### 数据流详细设计

#### 1. 消息分发日志结构

每条消息分发记录包含以下信息：

```rust
struct MessageDispatchLog {
    // 时间信息
    timestamp: DateTime<Utc>,          // 分发时间
    message_id: String,                 // 消息唯一标识

    // 消息源信息
    source_type: MessageSource,         // 消息源类型
    source_detail: String,              // 具体来源（系统模块/组织ID/审批流程ID）

    // 目标信息
    target_orgs: Vec<i64>,              // 目标组织ID列表
    target_roles: Vec<String>,          // 目标角色编码列表
    target_users: Vec<i64>,             // 目标用户ID列表

    // 消息分类
    msg_type: MessageType,              // 消息类型（系统/组织/任务）
    category: String,                   // 业务分类

    // 分发结果
    channels: Vec<String>,              // 使用的渠道
    status: DispatchStatus,             // 分发状态
}
```

#### 2. 日志文件格式

```
# 日志文件: logs/message-dispatch.2024-01-01.log

[2024-01-01T09:15:30.123Z] MSG_abc123 | SOURCE:system | FROM:system_monitor | TYPE:system_security | ORGS:[] | ROLES:[] | USERS:[1001,1002,1003] | CHANNELS:[websocket,log] | STATUS:success
[2024-01-01T09:20:45.456Z] MSG_def456 | SOURCE:organization | FROM:org_10 | TYPE:org_department | ORGS:[10,11] | ROLES:[] | USERS:[2001,2002,2003,2004] | CHANNELS:[websocket,log] | STATUS:success
[2024-01-01T09:25:12.789Z] MSG_ghi789 | SOURCE:workflow | FROM:wf_approval_123 | TYPE:workflow_todo | ORGS:[] | ROLES:[manager] | USERS:[3001] | CHANNELS:[websocket,log] | STATUS:success
```

#### 3. 渠道抽象接口

```rust
#[async_trait]
pub trait MessageChannel: Send + Sync {
    /// 渠道名称
    fn name(&self) -> &str;

    /// 是否支持该消息类型
    fn supports(&self, msg_type: &MessageType) -> bool;

    /// 发送消息
    async fn send(
        &self,
        message: &Message,
        target: &DispatchTarget,
    ) -> Result<ChannelResult, ChannelError>;
}

/// 分发目标
pub struct DispatchTarget {
    pub user_id: i64,
    pub org_id: Option<i64>,
    pub role_codes: Vec<String>,
    pub channels: Vec<String>,
}
```

---

## 数据库设计 (PostgreSQL)

### 1. 租户表 (tenants)

```sql
CREATE TABLE tenants (
    id BIGSERIAL PRIMARY KEY,
    tenant_code VARCHAR(50) UNIQUE NOT NULL,
    tenant_name VARCHAR(100) NOT NULL,
    status SMALLINT DEFAULT 1,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_tenants_code ON tenants(tenant_code);
CREATE INDEX idx_tenants_status ON tenants(status);

COMMENT ON TABLE tenants IS '租户表';
COMMENT ON COLUMN tenants.tenant_code IS '租户编码';
COMMENT ON COLUMN tenants.status IS '状态 1:正常 0:禁用';
```

### 说明：用户与组织数据来源

> 用户信息（`user_id`、`org_id`、`role_codes` 等）**不在本模块维护**，统一从 JWT Token 中解析获取。
>
> JWT Claims 结构示例：
> ```json
> {
>   "sub": "123456",
>   "tenant_id": 1,
>   "org_id": 10,
>   "org_path": "1/3/10",
>   "roles": ["admin", "teacher"],
>   "exp": 1700000000
> }
> ```
>
> - 消息发送时，`sender_id` / `tenant_id` 从 JWT 提取
> - 目标规则中的 `user_ids` / `org_ids` / `role_codes` 由调用方直接传入（通过外部用户/组织服务解析后传递）
> - 本模块仅负责消息的存储、路由和推送，不查询用户组织表

### 2. 消息模板表 (message_templates)

```sql
CREATE TABLE message_templates (
    id BIGSERIAL PRIMARY KEY,
    template_code VARCHAR(50) UNIQUE NOT NULL,
    template_name VARCHAR(100) NOT NULL,
    category VARCHAR(30) NOT NULL,
    priority SMALLINT DEFAULT 2,
    title_template TEXT,
    content_template TEXT,
    jump_type VARCHAR(20),
    jump_params JSONB,
    channels JSONB,
    is_system SMALLINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_message_templates_code ON message_templates(template_code);
CREATE INDEX idx_message_templates_category ON message_templates(category);

COMMENT ON TABLE message_templates IS '消息模板表';
COMMENT ON COLUMN message_templates.category IS '分类:system/business/alarm/interaction';
COMMENT ON COLUMN message_templates.priority IS '优先级 1:紧急 2:重要 3:普通 4:低优';
COMMENT ON COLUMN message_templates.title_template IS '标题模板 支持变量 {{var}}';
COMMENT ON COLUMN message_templates.jump_type IS '跳转类型:url/route/action';
COMMENT ON COLUMN message_templates.channels IS '推送渠道 ["web","email","dingtalk"]';
```

### 3. 消息表 (messages)

```sql
CREATE TABLE messages (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    message_code VARCHAR(50) UNIQUE NOT NULL,
    template_id BIGINT REFERENCES message_templates(id) ON DELETE SET NULL,
    category VARCHAR(30) NOT NULL,
    priority SMALLINT DEFAULT 2,
    title VARCHAR(200) NOT NULL,
    content TEXT,
    jump_type VARCHAR(20),
    jump_params JSONB,
    extra_data JSONB,
    send_type SMALLINT DEFAULT 1,
    scheduled_at TIMESTAMPTZ,
    sent_at TIMESTAMPTZ,
    expire_at TIMESTAMPTZ,
    sender_id BIGINT,  -- 来自 JWT，不维护外键
    sender_type VARCHAR(20) DEFAULT 'user',
    -- 消息源信息
    source_type VARCHAR(20) DEFAULT 'system',  -- 消息源类型: system/organization/workflow/external
    source_detail VARCHAR(100),                 -- 消息源详情（系统模块/组织ID/流程ID）
    status SMALLINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_messages_tenant ON messages(tenant_id);
CREATE INDEX idx_messages_template ON messages(template_id);
CREATE INDEX idx_messages_status ON messages(status);
CREATE INDEX idx_messages_scheduled ON messages(scheduled_at) WHERE scheduled_at IS NOT NULL;
CREATE INDEX idx_messages_code ON messages(message_code);
CREATE INDEX idx_messages_created ON messages(created_at DESC);

COMMENT ON TABLE messages IS '消息表';
COMMENT ON COLUMN messages.send_type IS '发送类型 1:立即 2:定时';
COMMENT ON COLUMN messages.sender_type IS 'user/system';
COMMENT ON COLUMN messages.source_type IS '消息源类型: system/organization/workflow/external';
COMMENT ON COLUMN messages.source_detail IS '消息源详情: 系统模块名/组织ID/审批流程ID';
COMMENT ON COLUMN messages.status IS '0:待发送 1:已发送 2:已取消 3:失败';
```

### 4. 消息接收规则表 (message_target_rules)

```sql
CREATE TABLE message_target_rules (
    id BIGSERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    target_type VARCHAR(20) NOT NULL,
    target_scope JSONB,
    filter_conditions JSONB,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_message_target_rules_message ON message_target_rules(message_id);

COMMENT ON TABLE message_target_rules IS '消息接收规则表';
COMMENT ON COLUMN message_target_rules.target_type IS '目标类型:user/org/role/custom';
COMMENT ON COLUMN message_target_rules.target_scope IS '目标范围配置';
COMMENT ON COLUMN message_target_rules.filter_conditions IS '筛选条件';
```

### 5. 用户消息表 (user_messages)

```sql
CREATE TABLE user_messages (
    id BIGSERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL,  -- 来自 JWT，不维护外键
    tenant_id BIGINT NOT NULL,
    is_read SMALLINT DEFAULT 0,
    read_at TIMESTAMPTZ,
    is_deleted SMALLINT DEFAULT 0,
    deleted_at TIMESTAMPTZ,
    is_pinned SMALLINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(message_id, user_id)
);

CREATE INDEX idx_user_messages_user_read ON user_messages(user_id, is_read, is_deleted);
CREATE INDEX idx_user_messages_tenant_user ON user_messages(tenant_id, user_id);
CREATE INDEX idx_user_messages_created ON user_messages(created_at DESC);

COMMENT ON TABLE user_messages IS '用户消息表';
COMMENT ON COLUMN user_messages.is_read IS '是否已读';
COMMENT ON COLUMN user_messages.is_deleted IS '是否删除';
COMMENT ON COLUMN user_messages.is_pinned IS '是否置顶';
```

### 6. 用户消息配置表 (user_message_settings)

```sql
CREATE TABLE user_message_settings (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,  -- 来自 JWT，不维护外键
    category VARCHAR(30),
    web_enabled SMALLINT DEFAULT 1,
    email_enabled SMALLINT DEFAULT 0,
    dingtalk_enabled SMALLINT DEFAULT 0,
    do_not_disturb SMALLINT DEFAULT 0,
    dnd_start_time TIME,
    dnd_end_time TIME,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, category)
);

COMMENT ON TABLE user_message_settings IS '用户消息配置表';
COMMENT ON COLUMN user_message_settings.web_enabled IS '站内消息开关';
COMMENT ON COLUMN user_message_settings.email_enabled IS '邮件通知开关';
COMMENT ON COLUMN user_message_settings.dingtalk_enabled IS '钉钉通知开关';
COMMENT ON COLUMN user_message_settings.do_not_disturb IS '免打扰模式';
```

### 7. 消息推送记录表 (message_push_logs)

```sql
CREATE TABLE message_push_logs (
    id BIGSERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    channel VARCHAR(20) NOT NULL,
    status SMALLINT,
    error_msg TEXT,
    pushed_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_message_push_logs_message ON message_push_logs(message_id);
CREATE INDEX idx_message_push_logs_user_channel ON message_push_logs(user_id, channel);
CREATE INDEX idx_message_push_logs_pushed ON message_push_logs(pushed_at DESC);

COMMENT ON TABLE message_push_logs IS '消息推送记录表';
COMMENT ON COLUMN message_push_logs.channel IS '推送渠道:web/email/dingtalk';
COMMENT ON COLUMN message_push_logs.status IS '1:成功 0:失败';
```

### 8. 触发器：自动更新 updated_at

```sql
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 为需要的表创建触发器
CREATE TRIGGER update_tenants_updated_at BEFORE UPDATE ON tenants
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_message_templates_updated_at BEFORE UPDATE ON message_templates
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_messages_updated_at BEFORE UPDATE ON messages
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_user_messages_updated_at BEFORE UPDATE ON user_messages
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_user_message_settings_updated_at BEFORE UPDATE ON user_message_settings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
```

---

## Cargo.toml

```toml
[package]
name = "message-system"
version = "0.1.0"
edition = "2021"

[dependencies]
# Web Framework
axum = { version = "0.7", features = ["ws", "macros"] }
tokio = { version = "1", features = ["full"] }
tower = { version = "0.4", features = ["util"] }
tower-http = { version = "0.5", features = ["cors", "trace"] }

# Database
sqlx = { version = "0.7", features = [
    "runtime-tokio-rustls",
    "postgres",
    "chrono",
    "uuid",
    "json",
    "migrate"
] }

# Redis
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }

# Message Queue
lapin = "2.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Authentication
jsonwebtoken = "9.2"

# Time
chrono = { version = "0.4", features = ["serde"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Configuration
config = "0.14"
dotenv = "0.15"

# Error Handling
anyhow = "1.0"
thiserror = "1.0"

# Utilities
uuid = { version = "1.6", features = ["v4", "serde"] }
async-trait = "0.1"
futures = "0.3"

# Template Engine (for message rendering)
tera = "1.19"

# Email
lettre = { version = "0.11", features = ["tokio1-rustls-tls"] }

# Task Scheduler
tokio-cron-scheduler = "0.10"

[dev-dependencies]
axum-test = "14.0"
```

---

## 配置文件

### .env

```env
# Server
SERVER_HOST=0.0.0.0
SERVER_PORT=3000

# Database
DATABASE_URL=postgresql://postgres:password@localhost:5432/message_system

# Redis
REDIS_URL=redis://localhost:6379

# RabbitMQ
RABBITMQ_URL=amqp://guest:guest@localhost:5672

# JWT
JWT_SECRET=your-secret-key-here
JWT_EXPIRATION=86400

# SMTP (Email)
SMTP_HOST=smtp.gmail.com
SMTP_PORT=587
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-password

# DingTalk
DINGTALK_WEBHOOK=https://oapi.dingtalk.com/robot/send?access_token=xxx

# Log Level
RUST_LOG=info,message_system=debug
```

---

## 核心代码实现

### src/main.rs

```rust
use axum::{
    routing::{get, post, put, delete},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod models;
mod handlers;
mod services;
mod repositories;
mod websocket;
mod queue;
mod cache;
mod middleware;
mod utils;

use config::Config;
use websocket::WebSocketManager;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub redis: redis::aio::ConnectionManager,
    pub ws_manager: Arc<RwLock<WebSocketManager>>,
    pub config: Arc<Config>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,message_system=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 加载配置
    dotenv::dotenv().ok();
    let config = Config::from_env()?;

    // 连接数据库
    let db = PgPoolOptions::new()
        .max_connections(50)
        .connect(&config.database_url)
        .await?;

    // 运行迁移
    sqlx::migrate!("./migrations").run(&db).await?;

    // 连接 Redis
    let redis_client = redis::Client::open(config.redis_url.as_str())?;
    let redis = redis::aio::ConnectionManager::new(redis_client).await?;

    // 创建 WebSocket 管理器
    let ws_manager = Arc::new(RwLock::new(WebSocketManager::new()));

    // 创建应用状态
    let state = AppState {
        db: db.clone(),
        redis: redis.clone(),
        ws_manager: ws_manager.clone(),
        config: Arc::new(config.clone()),
    };

    // 启动消息队列消费者
    tokio::spawn(async move {
        if let Err(e) = queue::consumer::start_consumer(
            config.clone(),
            db,
            redis,
            ws_manager,
        ).await {
            tracing::error!("消息队列消费者错误: {:?}", e);
        }
    });

    // 启动定时任务
    tokio::spawn(async {
        services::scheduler::start_scheduler().await;
    });

    // 构建路由
    let app = Router::new()
        // 消息模板路由
        .route("/api/message-templates", post(handlers::template_handler::create_template))
        .route("/api/message-templates", get(handlers::template_handler::list_templates))
        .route("/api/message-templates/:code", get(handlers::template_handler::get_template))
        .route("/api/message-templates/:id", put(handlers::template_handler::update_template))
        .route("/api/message-templates/:id", delete(handlers::template_handler::delete_template))
        
        // 消息发送路由
        .route("/api/messages/send", post(handlers::message_handler::send_message))
        
        // 用户消息路由
        .route("/api/messages", get(handlers::message_handler::list_user_messages))
        .route("/api/messages/:id", get(handlers::message_handler::get_message_detail))
        .route("/api/messages/:id/read", post(handlers::message_handler::mark_as_read))
        .route("/api/messages/batch-read", post(handlers::message_handler::batch_mark_as_read))
        .route("/api/messages/read-by-category", post(handlers::message_handler::mark_category_as_read))
        .route("/api/messages/read-all", post(handlers::message_handler::mark_all_as_read))
        .route("/api/messages/:id", delete(handlers::message_handler::delete_message))
        .route("/api/messages/batch-delete", post(handlers::message_handler::batch_delete))
        .route("/api/messages/:id/pin", post(handlers::message_handler::pin_message))
        .route("/api/messages/:id/pin", delete(handlers::message_handler::unpin_message))
        .route("/api/messages/unread-count", get(handlers::message_handler::get_unread_count))
        .route("/api/messages/unread-stats", get(handlers::message_handler::get_unread_stats))
        
        // 用户设置路由
        .route("/api/message-settings", get(handlers::setting_handler::get_settings))
        .route("/api/message-settings", put(handlers::setting_handler::update_settings))
        .route("/api/message-settings/dnd", put(handlers::setting_handler::update_dnd))
        .route("/api/message-settings/channels", put(handlers::setting_handler::update_channels))
        
        // 管理员路由
        .route("/api/admin/messages", get(handlers::admin_handler::list_all_messages))
        .route("/api/admin/messages/:id/details", get(handlers::admin_handler::get_message_details))
        .route("/api/admin/messages/:id/push-logs", get(handlers::admin_handler::get_push_logs))
        .route("/api/admin/messages/:id/revoke", post(handlers::admin_handler::revoke_message))
        .route("/api/admin/messages/:id/cancel", post(handlers::admin_handler::cancel_message))
        .route("/api/admin/messages/:id/retry", post(handlers::admin_handler::retry_message))
        .route("/api/admin/messages/stats", get(handlers::admin_handler::get_stats))
        
        // WebSocket 路由
        .route("/ws/:tenant_id", get(websocket::handler::ws_handler))
        
        // 健康检查
        .route("/health", get(|| async { "OK" }))
        
        .layer(CorsLayer::permissive())
        .layer(middleware::auth::auth_middleware())
        .with_state(state);

    // 启动服务器
    let addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    tracing::info!("服务器启动在 {}", addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}
```

### src/config.rs

```rust
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub rabbitmq_url: String,
    pub jwt_secret: String,
    pub jwt_expiration: i64,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub dingtalk_webhook: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Config {
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()?,
            database_url: env::var("DATABASE_URL")?,
            redis_url: env::var("REDIS_URL")?,
            rabbitmq_url: env::var("RABBITMQ_URL")?,
            jwt_secret: env::var("JWT_SECRET")?,
            jwt_expiration: env::var("JWT_EXPIRATION")
                .unwrap_or_else(|_| "86400".to_string())
                .parse()?,
            smtp_host: env::var("SMTP_HOST").unwrap_or_default(),
            smtp_port: env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()?,
            smtp_username: env::var("SMTP_USERNAME").unwrap_or_default(),
            smtp_password: env::var("SMTP_PASSWORD").unwrap_or_default(),
            dingtalk_webhook: env::var("DINGTALK_WEBHOOK").unwrap_or_default(),
        })
    }
}
```

### src/error.rs

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Redis错误: {0}")]
    Redis(#[from] redis::RedisError),
    
    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("未授权")]
    Unauthorized,
    
    #[error("资源未找到")]
    NotFound,
    
    #[error("无效的请求: {0}")]
    BadRequest(String),
    
    #[error("内部服务器错误: {0}")]
    Internal(String),
    
    #[error("模板渲染错误: {0}")]
    TemplateRender(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Database(e) => {
                tracing::error!("数据库错误: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "数据库错误".to_string())
            }
            AppError::Redis(e) => {
                tracing::error!("Redis错误: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "缓存错误".to_string())
            }
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "未授权".to_string()),
            AppError::NotFound => (StatusCode::NOT_FOUND, "资源未找到".to_string()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
```

### src/models/message.rs

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: i64,
    pub tenant_id: i64,
    pub message_code: String,
    pub template_id: Option<i64>,
    pub category: String,
    pub priority: i16,
    pub title: String,
    pub content: Option<String>,
    pub jump_type: Option<String>,
    pub jump_params: Option<serde_json::Value>,
    pub extra_data: Option<serde_json::Value>,
    pub send_type: i16,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub sent_at: Option<DateTime<Utc>>,
    pub expire_at: Option<DateTime<Utc>>,
    pub sender_id: Option<i64>,
    pub sender_type: String,
    pub status: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    pub template_code: String,
    pub target_rules: Vec<TargetRule>,
    pub variables: serde_json::Value,
    pub send_type: Option<i16>,
    pub scheduled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TargetRule {
    pub target_type: String,
    pub target_scope: serde_json::Value,
    pub filter_conditions: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetScope {
    pub user_ids: Option<Vec<i64>>,
    pub org_ids: Option<Vec<i64>>,
    pub include_children: Option<bool>,
    pub role_codes: Option<Vec<String>>,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserMessage {
    pub id: i64,
    pub message_id: i64,
    pub user_id: i64,
    pub tenant_id: i64,
    pub is_read: i16,
    pub read_at: Option<DateTime<Utc>>,
    pub is_deleted: i16,
    pub deleted_at: Option<DateTime<Utc>>,
    pub is_pinned: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessageDetail {
    #[serde(flatten)]
    pub message: Message,
    pub is_read: i16,
    pub read_at: Option<DateTime<Utc>>,
    pub is_pinned: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageListQuery {
    pub category: Option<String>,
    pub is_read: Option<i16>,
    pub priority: Option<i16>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnreadStats {
    pub total: i64,
    pub by_category: Vec<CategoryCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
}
```

### src/models/message_template.rs

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MessageTemplate {
    pub id: i64,
    pub template_code: String,
    pub template_name: String,
    pub category: String,
    pub priority: i16,
    pub title_template: Option<String>,
    pub content_template: Option<String>,
    pub jump_type: Option<String>,
    pub jump_params: Option<serde_json::Value>,
    pub channels: Option<serde_json::Value>,
    pub is_system: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTemplateRequest {
    pub template_code: String,
    pub template_name: String,
    pub category: String,
    pub priority: Option<i16>,
    pub title_template: Option<String>,
    pub content_template: Option<String>,
    pub jump_type: Option<String>,
    pub jump_params: Option<serde_json::Value>,
    pub channels: Option<Vec<String>>,
}
```

### src/services/message_service.rs

```rust
use crate::{
    error::{AppError, AppResult},
    models::message::{CreateMessageRequest, Message, TargetRule},
    repositories::message_repository::MessageRepository,
    services::{
        target_resolver::TargetResolver,
        template_service::TemplateService,
    },
    queue::producer::MessageProducer,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct MessageService {
    repo: MessageRepository,
    template_service: TemplateService,
    target_resolver: TargetResolver,
    producer: MessageProducer,
}

impl MessageService {
    pub fn new(
        db: PgPool,
        redis: redis::aio::ConnectionManager,
        producer: MessageProducer,
    ) -> Self {
        Self {
            repo: MessageRepository::new(db.clone()),
            template_service: TemplateService::new(db.clone()),
            target_resolver: TargetResolver::new(db, redis),
            producer,
        }
    }

    pub async fn send_message(
        &self,
        tenant_id: i64,      // 从 JWT 提取
        sender_id: i64,      // 从 JWT 提取
        request: CreateMessageRequest,
    ) -> AppResult<i64> {
        // 1. 获取模板
        let template = self.template_service
            .get_by_code(&request.template_code)
            .await?
            .ok_or_else(|| AppError::NotFound)?;

        // 2. 渲染消息内容
        let title = self.render_template(
            &template.title_template.unwrap_or_default(),
            &request.variables,
        )?;
        let content = self.render_template(
            &template.content_template.unwrap_or_default(),
            &request.variables,
        )?;

        // 3. 创建消息
        let message_code = format!("MSG_{}", Uuid::new_v4());
        let send_type = request.send_type.unwrap_or(1);
        
        let message_id = self.repo.create(
            tenant_id,
            message_code.clone(),
            template.id,
            template.category.clone(),
            template.priority,
            title,
            Some(content),
            template.jump_type.clone(),
            template.jump_params.clone(),
            None,
            send_type,
            request.scheduled_at,
            Some(sender_id),
        ).await?;

        // 4. 保存目标规则
        for rule in request.target_rules {
            self.repo.create_target_rule(
                message_id,
                rule.target_type,
                rule.target_scope,
                rule.filter_conditions,
            ).await?;
        }

        // 5. 加入消息队列
        if send_type == 1 {
            // 立即发送
            self.producer.publish_message(message_id).await?;
        } else {
            // 定时发送
            let delay = request.scheduled_at
                .map(|t| (t.timestamp() - chrono::Utc::now().timestamp()).max(0) as u64)
                .unwrap_or(0);
            self.producer.publish_message_with_delay(message_id, delay).await?;
        }

        Ok(message_id)
    }

    fn render_template(
        &self,
        template: &str,
        variables: &serde_json::Value,
    ) -> AppResult<String> {
        let mut tera = tera::Tera::default();
        tera.add_raw_template("msg", template)
            .map_err(|e| AppError::TemplateRender(e.to_string()))?;

        let context = tera::Context::from_serialize(variables)
            .map_err(|e| AppError::TemplateRender(e.to_string()))?;

        tera.render("msg", &context)
            .map_err(|e| AppError::TemplateRender(e.to_string()))
    }
}
```

### src/services/target_resolver.rs

```rust
use crate::{
    error::{AppError, AppResult},
    models::message::{TargetRule, TargetScope},
};
use std::collections::HashSet;

/// 目标用户解析器
///
/// ⚠️ 用户/组织/角色数据由调用方从外部服务获取并直接传入
/// 本模块不维护用户/组织表，仅解析 target_rules 中指定的用户ID列表
pub struct TargetResolver;

impl TargetResolver {
    pub fn new() -> Self {
        Self
    }

    pub async fn resolve_target_users(
        &self,
        rules: Vec<TargetRule>,
    ) -> AppResult<Vec<i64>> {
        let mut all_user_ids = HashSet::new();

        for rule in rules {
            let scope: TargetScope = serde_json::from_value(rule.target_scope)
                .map_err(|e| AppError::BadRequest(e.to_string()))?;

            let user_ids = match rule.target_type.as_str() {
                "user" => self.resolve_user_target(&scope),
                "org" => self.resolve_org_target(&scope),
                "role" => self.resolve_role_target(&scope),
                "custom" => self.resolve_custom_target(&scope),
                _ => vec![],
            };

            all_user_ids.extend(user_ids);
        }

        Ok(all_user_ids.into_iter().collect())
    }

    /// 直接指定用户ID列表
    fn resolve_user_target(&self, scope: &TargetScope) -> Vec<i64> {
        scope.user_ids.clone().unwrap_or_default()
    }

    /// 组织目标：org_ids 由调用方（外部服务）解析完成后传入
    fn resolve_org_target(&self, scope: &TargetScope) -> Vec<i64> {
        // 外部服务已根据 org_ids 和 include_children 解析出所有用户
        scope.user_ids.clone().unwrap_or_default()
    }

    /// 角色目标：role_codes 由调用方（外部服务）解析完成后传入
    fn resolve_role_target(&self, scope: &TargetScope) -> Vec<i64> {
        // 外部服务已根据 role_codes 解析出所有用户
        scope.user_ids.clone().unwrap_or_default()
    }

    /// 自定义条件：由调用方（外部服务）解析完成后传入
    fn resolve_custom_target(&self, scope: &TargetScope) -> Vec<i64> {
        // 外部服务已根据自定义条件解析出所有用户
        scope.user_ids.clone().unwrap_or_default()
    }
}
```

### src/services/push_service.rs

```rust
use crate::{
    error::AppResult,
    models::message::Message,
    repositories::message_repository::MessageRepository,
    websocket::WebSocketManager,
    cache::redis_client::RedisCache,
};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PushService {
    message_repo: MessageRepository,
    ws_manager: Arc<RwLock<WebSocketManager>>,
    cache: RedisCache,
}

impl PushService {
    pub fn new(
        db: PgPool,
        redis: redis::aio::ConnectionManager,
        ws_manager: Arc<RwLock<WebSocketManager>>,
    ) -> Self {
        Self {
            message_repo: MessageRepository::new(db),
            ws_manager,
            cache: RedisCache::new(redis),
        }
    }

    pub async fn push_to_users(
        &self,
        message: &Message,
        user_ids: Vec<i64>,
    ) -> AppResult<()> {
        // 1. 批量创建用户消息记录
        self.message_repo.create_user_messages(message.id, &user_ids).await?;

        // 2. 检查用户在线状态并推送
        for user_id in user_ids {
            // 更新未读数缓存
            self.cache.increment_unread(user_id, &message.category).await?;

            // 检查用户是否在线
            let is_online = self.cache
                .is_user_online(message.tenant_id, user_id)
                .await?;

            if is_online {
                // WebSocket 实时推送
                self.push_via_websocket(message, user_id).await?;
            } else {
                // 离线推送（邮件、钉钉等）
                self.push_offline_channels(message, user_id).await?;
            }

            // 记录推送日志
            self.message_repo.log_push(
                message.id,
                user_id,
                "web",
                if is_online { 1 } else { 0 },
                None,
            ).await?;
        }

        // 3. 更新消息状态
        self.message_repo.update_status(
            message.id,
            1, // 已发送
            Some(chrono::Utc::now()),
        ).await?;

        Ok(())
    }

    async fn push_via_websocket(
        &self,
        message: &Message,
        user_id: i64,
    ) -> AppResult<()> {
        let payload = serde_json::json!({
            "id": message.id,
            "title": message.title,
            "category": message.category,
            "priority": message.priority,
            "created_at": message.created_at,
        });

        let manager = self.ws_manager.read().await;
        manager.send_to_user(message.tenant_id, user_id, payload).await;

        Ok(())
    }

    async fn push_offline_channels(
        &self,
        message: &Message,
        user_id: i64,
    ) -> AppResult<()> {
        // TODO: 从消息配置或用户配置中获取推送渠道设置
        // 用户信息从 JWT 传入，如需详细配置需调用外部用户服务 API
        // - 邮件
        // - 钉钉
        // - 企业微信
        // - 短信

        tracing::debug!("离线推送: user={}, message={}", user_id, message.id);

        Ok(())
    }
}
```

### src/websocket/handler.rs

```rust
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use crate::{AppState, error::AppResult};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(tenant_id): Path<i64>,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, tenant_id, state))
}

async fn handle_socket(socket: WebSocket, tenant_id: i64, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // TODO: 从连接参数中提取 user_id 和 token
    let user_id = 1; // 示例

    // 注册连接
    {
        let mut manager = state.ws_manager.write().await;
        manager.add_connection(tenant_id, user_id, sender.clone()).await;
    }

    // 更新 Redis 在线状态
    let _ = state.cache.set_user_online(tenant_id, user_id).await;

    // 处理接收的消息
    while let Some(msg) = receiver.next().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    tracing::debug!("收到消息: {}", text);
                    // 处理心跳等
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }
    }

    // 移除连接
    {
        let mut manager = state.ws_manager.write().await;
        manager.remove_connection(tenant_id, user_id).await;
    }

    // 更新 Redis 离线状态
    let _ = state.cache.set_user_offline(tenant_id, user_id).await;
}
```

### src/websocket/manager.rs

```rust
use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub struct WebSocketManager {
    // tenant_id -> user_id -> sender
    connections: HashMap<i64, HashMap<i64, mpsc::UnboundedSender<Message>>>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    pub async fn add_connection(
        &mut self,
        tenant_id: i64,
        user_id: i64,
        mut sender: SplitSink<WebSocket, Message>,
    ) {
        let (tx, mut rx) = mpsc::unbounded_channel();

        // 启动发送任务
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if sender.send(msg).await.is_err() {
                    break;
                }
            }
        });

        self.connections
            .entry(tenant_id)
            .or_insert_with(HashMap::new)
            .insert(user_id, tx);

        tracing::info!("WebSocket 连接建立: tenant={}, user={}", tenant_id, user_id);
    }

    pub async fn remove_connection(&mut self, tenant_id: i64, user_id: i64) {
        if let Some(tenant_conns) = self.connections.get_mut(&tenant_id) {
            tenant_conns.remove(&user_id);
            
            if tenant_conns.is_empty() {
                self.connections.remove(&tenant_id);
            }
        }

        tracing::info!("WebSocket 连接断开: tenant={}, user={}", tenant_id, user_id);
    }

    pub async fn send_to_user(
        &self,
        tenant_id: i64,
        user_id: i64,
        payload: serde_json::Value,
    ) {
        if let Some(tenant_conns) = self.connections.get(&tenant_id) {
            if let Some(tx) = tenant_conns.get(&user_id) {
                let msg = Message::Text(serde_json::to_string(&payload).unwrap());
                let _ = tx.send(msg);
            }
        }
    }

    pub async fn broadcast_to_tenant(
        &self,
        tenant_id: i64,
        payload: serde_json::Value,
    ) {
        if let Some(tenant_conns) = self.connections.get(&tenant_id) {
            let msg = Message::Text(serde_json::to_string(&payload).unwrap());
            
            for tx in tenant_conns.values() {
                let _ = tx.send(msg.clone());
            }
        }
    }
}
```

### src/queue/consumer.rs

```rust
use crate::{
    config::Config,
    services::{
        push_service::PushService,
        target_resolver::TargetResolver,
    },
    repositories::message_repository::MessageRepository,
    websocket::WebSocketManager,
};
use lapin::{
    options::*,
    types::FieldTable,
    Connection, ConnectionProperties,
};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn start_consumer(
    config: Config,
    db: sqlx::PgPool,
    redis: redis::aio::ConnectionManager,
    ws_manager: Arc<RwLock<WebSocketManager>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 连接 RabbitMQ
    let conn = Connection::connect(
        &config.rabbitmq_url,
        ConnectionProperties::default(),
    ).await?;

    let channel = conn.create_channel().await?;

    // 声明队列
    let queue = channel
        .queue_declare(
            "message_queue",
            QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await?;

    tracing::info!("消息队列消费者已启动: {}", queue.name());

    // 创建服务实例
    let message_repo = MessageRepository::new(db.clone());
    let target_resolver = TargetResolver::new();
    let push_service = PushService::new(db, redis, ws_manager);

    // 开始消费
    let mut consumer = channel
        .basic_consume(
            "message_queue",
            "message_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    while let Some(delivery) = consumer.next().await {
        if let Ok(delivery) = delivery {
            let data = String::from_utf8_lossy(&delivery.data);
            
            if let Ok(message_id) = data.parse::<i64>() {
                tracing::info!("处理消息: {}", message_id);

                // 处理消息
                match process_message(
                    message_id,
                    &message_repo,
                    &target_resolver,
                    &push_service,
                ).await {
                    Ok(_) => {
                        // ACK
                        delivery.ack(BasicAckOptions::default()).await?;
                    }
                    Err(e) => {
                        tracing::error!("处理消息失败: {:?}", e);
                        // NACK
                        delivery.nack(BasicNackOptions::default()).await?;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn process_message(
    message_id: i64,
    message_repo: &MessageRepository,
    target_resolver: &TargetResolver,
    push_service: &PushService,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 获取消息
    let message = message_repo.get_by_id(message_id).await?
        .ok_or("消息不存在")?;

    // 2. 获取目标规则
    let rules = message_repo.get_target_rules(message_id).await?;

    // 3. 解析目标用户
    let user_ids = target_resolver.resolve_target_users(rules).await?;

    tracing::info!("消息 {} 目标用户数: {}", message_id, user_ids.len());

    // 4. 推送消息
    push_service.push_to_users(&message, user_ids).await?;

    Ok(())
}
```

### src/handlers/message_handler.rs

```rust
use axum::{
    extract::{Path, Query, State},
    Json,
};
use crate::{
    AppState,
    error::{AppError, AppResult},
    models::message::{CreateMessageRequest, MessageListQuery, UserMessageDetail},
    services::message_service::MessageService,
    repositories::message_repository::MessageRepository,
    utils::pagination::PaginatedResponse,
};

pub async fn send_message(
    State(state): State<AppState>,
    // TODO: 从 JWT Claims 中提取 tenant_id 和 user_id
    // claims: Claims,  // axum extractor
    Json(request): Json<CreateMessageRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let tenant_id = 1;  // 从 claims.tenant_id 获取
    let user_id = 1;    // 从 claims.sub 获取

    let producer = crate::queue::producer::MessageProducer::new(&state.config)?;
    let service = MessageService::new(state.db, state.redis, producer);
    
    let message_id = service.send_message(tenant_id, user_id, request).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message_id": message_id
    })))
}

pub async fn list_user_messages(
    State(state): State<AppState>,
    // TODO: 从 JWT Claims 提取 user_id
    Query(query): Query<MessageListQuery>,
) -> AppResult<Json<PaginatedResponse<UserMessageDetail>>> {
    let user_id = 1;  // 从 claims.sub 获取

    let repo = MessageRepository::new(state.db);
    
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    
    let messages = repo.list_user_messages(
        user_id,
        query.category.as_deref(),
        query.is_read,
        page,
        page_size,
    ).await?;

    let total = repo.count_user_messages(
        user_id,
        query.category.as_deref(),
        query.is_read,
    ).await?;

    Ok(Json(PaginatedResponse {
        data: messages,
        total,
        page,
        page_size,
    }))
}

pub async fn mark_as_read(
    State(state): State<AppState>,
    // TODO: 从 JWT Claims 提取 user_id
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = 1;  // 从 claims.sub 获取

    let repo = MessageRepository::new(state.db.clone());
    repo.mark_as_read(id, user_id).await?;

    // 更新 Redis 未读数
    let message = repo.get_by_id(id).await?
        .ok_or(AppError::NotFound)?;
    
    crate::cache::redis_client::RedisCache::new(state.redis)
        .decrement_unread(user_id, &message.category)
        .await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn batch_mark_as_read(
    State(state): State<AppState>,
    // TODO: 从 JWT Claims 提取 user_id
    Json(payload): Json<serde_json::Value>,
) -> AppResult<Json<serde_json::Value>> {
    let user_id = 1;  // 从 claims.sub 获取
    
    let message_ids: Vec<i64> = serde_json::from_value(
        payload.get("message_ids").cloned().unwrap_or_default()
    )?;

    let repo = MessageRepository::new(state.db);
    repo.batch_mark_as_read(&message_ids, user_id).await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn get_unread_stats(
    State(state): State<AppState>,
    // TODO: 从 JWT Claims 提取 user_id
) -> AppResult<Json<serde_json::Value>> {
    let user_id = 1;  // 从 claims.sub 获取

    let cache = crate::cache::redis_client::RedisCache::new(state.redis);
    let stats = cache.get_unread_stats(user_id).await?;

    Ok(Json(stats))
}

// 其他 handler 函数...
```

### src/repositories/message_repository.rs

```rust
use crate::{
    error::AppResult,
    models::message::{Message, TargetRule, UserMessage, UserMessageDetail},
};
use sqlx::PgPool;

pub struct MessageRepository {
    db: PgPool,
}

impl MessageRepository {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn create(
        &self,
        tenant_id: i64,
        message_code: String,
        template_id: i64,
        category: String,
        priority: i16,
        title: String,
        content: Option<String>,
        jump_type: Option<String>,
        jump_params: Option<serde_json::Value>,
        extra_data: Option<serde_json::Value>,
        send_type: i16,
        scheduled_at: Option<chrono::DateTime<chrono::Utc>>,
        sender_id: Option<i64>,
    ) -> AppResult<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO messages (
                tenant_id, message_code, template_id, category, priority,
                title, content, jump_type, jump_params, extra_data,
                send_type, scheduled_at, sender_id, status
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 0)
            RETURNING id
            "#,
            tenant_id,
            message_code,
            template_id,
            category,
            priority,
            title,
            content,
            jump_type,
            jump_params,
            extra_data,
            send_type,
            scheduled_at,
            sender_id,
        )
        .fetch_one(&self.db)
        .await?;

        Ok(result.id)
    }

    pub async fn create_target_rule(
        &self,
        message_id: i64,
        target_type: String,
        target_scope: serde_json::Value,
        filter_conditions: Option<serde_json::Value>,
    ) -> AppResult<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO message_target_rules (
                message_id, target_type, target_scope, filter_conditions
            ) VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
            message_id,
            target_type,
            target_scope,
            filter_conditions,
        )
        .fetch_one(&self.db)
        .await?;

        Ok(result.id)
    }

    pub async fn get_by_id(&self, id: i64) -> AppResult<Option<Message>> {
        let message = sqlx::query_as!(
            Message,
            "SELECT * FROM messages WHERE id = $1",
            id
        )
        .fetch_optional(&self.db)
        .await?;

        Ok(message)
    }

    pub async fn get_target_rules(&self, message_id: i64) -> AppResult<Vec<TargetRule>> {
        let rules = sqlx::query_as!(
            TargetRule,
            "SELECT target_type, target_scope, filter_conditions FROM message_target_rules WHERE message_id = $1",
            message_id
        )
        .fetch_all(&self.db)
        .await?;

        Ok(rules)
    }

    pub async fn create_user_messages(
        &self,
        message_id: i64,
        user_ids: &[i64],
    ) -> AppResult<()> {
        // 批量插入
        for user_id in user_ids {
            sqlx::query!(
                r#"
                INSERT INTO user_messages (message_id, user_id, tenant_id)
                SELECT $1, $2, tenant_id FROM messages WHERE id = $1
                ON CONFLICT (message_id, user_id) DO NOTHING
                "#,
                message_id,
                user_id,
            )
            .execute(&self.db)
            .await?;
        }

        Ok(())
    }

    pub async fn list_user_messages(
        &self,
        user_id: i64,
        category: Option<&str>,
        is_read: Option<i16>,
        page: i64,
        page_size: i64,
    ) -> AppResult<Vec<UserMessageDetail>> {
        let offset = (page - 1) * page_size;

        let messages = sqlx::query_as!(
            UserMessageDetail,
            r#"
            SELECT 
                m.*,
                um.is_read,
                um.read_at,
                um.is_pinned
            FROM user_messages um
            JOIN messages m ON um.message_id = m.id
            WHERE um.user_id = $1
              AND um.is_deleted = 0
              AND ($2::VARCHAR IS NULL OR m.category = $2)
              AND ($3::SMALLINT IS NULL OR um.is_read = $3)
            ORDER BY um.is_pinned DESC, m.created_at DESC
            LIMIT $4 OFFSET $5
            "#,
            user_id,
            category,
            is_read,
            page_size,
            offset,
        )
        .fetch_all(&self.db)
        .await?;

        Ok(messages)
    }

    pub async fn mark_as_read(&self, message_id: i64, user_id: i64) -> AppResult<()> {
        sqlx::query!(
            r#"
            UPDATE user_messages 
            SET is_read = 1, read_at = NOW()
            WHERE message_id = $1 AND user_id = $2
            "#,
            message_id,
            user_id,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn batch_mark_as_read(&self, message_ids: &[i64], user_id: i64) -> AppResult<()> {
        sqlx::query!(
            r#"
            UPDATE user_messages 
            SET is_read = 1, read_at = NOW()
            WHERE message_id = ANY($1) AND user_id = $2
            "#,
            message_ids,
            user_id,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn update_status(
        &self,
        message_id: i64,
        status: i16,
        sent_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> AppResult<()> {
        sqlx::query!(
            r#"
            UPDATE messages 
            SET status = $1, sent_at = $2
            WHERE id = $3
            "#,
            status,
            sent_at,
            message_id,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn log_push(
        &self,
        message_id: i64,
        user_id: i64,
        channel: &str,
        status: i16,
        error_msg: Option<&str>,
    ) -> AppResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO message_push_logs (message_id, user_id, channel, status, error_msg)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            message_id,
            user_id,
            channel,
            status,
            error_msg,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn count_user_messages(
        &self,
        user_id: i64,
        category: Option<&str>,
        is_read: Option<i16>,
    ) -> AppResult<i64> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM user_messages um
            JOIN messages m ON um.message_id = m.id
            WHERE um.user_id = $1
              AND um.is_deleted = 0
              AND ($2::VARCHAR IS NULL OR m.category = $2)
              AND ($3::SMALLINT IS NULL OR um.is_read = $3)
            "#,
            user_id,
            category,
            is_read,
        )
        .fetch_one(&self.db)
        .await?;

        Ok(result.count.unwrap_or(0))
    }
}
```

### src/cache/redis_client.rs

```rust
use crate::error::AppResult;
use redis::AsyncCommands;

pub struct RedisCache {
    conn: redis::aio::ConnectionManager,
}

impl RedisCache {
    pub fn new(conn: redis::aio::ConnectionManager) -> Self {
        Self { conn }
    }

    pub async fn set_user_online(&mut self, tenant_id: i64, user_id: i64) -> AppResult<()> {
        let key = format!("online:tenant:{}", tenant_id);
        self.conn.sadd::<_, _, ()>(key, user_id).await?;
        Ok(())
    }

    pub async fn set_user_offline(&mut self, tenant_id: i64, user_id: i64) -> AppResult<()> {
        let key = format!("online:tenant:{}", tenant_id);
        self.conn.srem::<_, _, ()>(key, user_id).await?;
        Ok(())
    }

    pub async fn is_user_online(&mut self, tenant_id: i64, user_id: i64) -> AppResult<bool> {
        let key = format!("online:tenant:{}", tenant_id);
        let result: bool = self.conn.sismember(key, user_id).await?;
        Ok(result)
    }

    pub async fn increment_unread(&mut self, user_id: i64, category: &str) -> AppResult<()> {
        let key = format!("unread:{}", user_id);
        self.conn.hincr::<_, _, _, ()>(key, category, 1).await?;
        Ok(())
    }

    pub async fn decrement_unread(&mut self, user_id: i64, category: &str) -> AppResult<()> {
        let key = format!("unread:{}", user_id);
        self.conn.hincr::<_, _, _, ()>(key, category, -1).await?;
        Ok(())
    }

    pub async fn get_unread_stats(&mut self, user_id: i64) -> AppResult<serde_json::Value> {
        let key = format!("unread:{}", user_id);
        let result: std::collections::HashMap<String, i64> = self.conn.hgetall(key).await?;
        
        let total: i64 = result.values().sum();
        
        Ok(serde_json::json!({
            "total": total,
            "by_category": result,
        }))
    }
}
```

---

## 运行说明

### 1. 环境准备

```bash
# 安装 PostgreSQL
docker run -d \
  --name postgres \
  -e POSTGRES_PASSWORD=password \
  -e POSTGRES_DB=message_system \
  -p 5432:5432 \
  postgres:15

# 安装 Redis
docker run -d \
  --name redis \
  -p 6379:6379 \
  redis:7

# 安装 RabbitMQ
docker run -d \
  --name rabbitmq \
  -p 5672:5672 \
  -p 15672:15672 \
  rabbitmq:3-management
```

### 2. 项目运行

```bash
# 克隆/创建项目
cargo new message-system
cd message-system

# 复制配置文件
cp .env.example .env
# 编辑 .env 填入正确的配置

# 运行数据库迁移
sqlx migrate run

# 开发模式运行
cargo run

# 生产模式构建
cargo build --release
./target/release/message-system
```

### 3. 数据库迁移

```bash
# 创建新的迁移
sqlx migrate add create_tenants

# 运行迁移
sqlx migrate run

# 回滚迁移
sqlx migrate revert
```

---

## 性能优化建议

1. **数据库优化**
   - 使用连接池（已实现）
   - 添加适当的索引（已实现）
   - 使用 EXPLAIN ANALYZE 分析慢查询
   - 考虑分表策略（按租户或时间）

2. **缓存策略**
   - 未读数使用 Redis Hash
   - 在线状态使用 Redis Set
   - 热点消息使用 Redis String
   - 设置合理的过期时间

3. **并发处理**
   - 使用 Tokio 异步运行时
   - WebSocket 连接池化
   - 消息队列批量处理
   - 使用 Arc + RwLock 共享状态

4. **监控和日志**
   - 使用 tracing 记录详细日志
   - 接入 Prometheus + Grafana
   - 设置告警规则
   - 定期性能分析

---

## 扩展功能

1. **多渠道推送**
   - 邮件（SMTP）
   - 钉钉机器人
   - 企业微信
   - 短信（阿里云 SMS）

2. **高级特性**
   - 消息模板变量校验
   - 消息发送频率限制
   - 消息阅后即焚
   - 消息撤回功能

3. **统计分析**
   - 消息发送量统计
   - 用户活跃度分析
   - 推送渠道效果对比
   - 消息阅读率统计

---

## 总结

本方案基于 Rust 生态构建了一个完整的多租户系统消息平台，具有以下特点：

- ✅ 高性能：Rust + Tokio 异步运行时
- ✅ 类型安全：编译时 SQL 检查（SQLx）
- ✅ 多租户隔离：命名空间 + 数据隔离
- ✅ 实时推送：WebSocket + Redis
- ✅ 认证解耦：用户信息从 JWT 获取，不维护用户/组织表
- ✅ 可扩展：消息队列 + 微服务架构
- ✅ 易维护：清晰的分层架构

可根据实际业务需求进行定制和扩展。
