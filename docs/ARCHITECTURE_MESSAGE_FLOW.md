# 消息系统数据流架构方案

## 1. 架构概述

本文档描述消息系统中**消息源**与**消息目标**之间的完整数据流架构。

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            消息分发数据流架构                                │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                              消息源层 (Source)                               │
├─────────────────┬─────────────────┬─────────────────┬───────────────────────┤
│   系统源        │   组织源        │   审批任务源    │    外部集成源         │
│  (System)       │  (Organization) │  (Workflow)     │   (External)          │
├─────────────────┼─────────────────┼─────────────────┼───────────────────────┤
│ • 系统公告      │ • 部门通知      │ • 待办任务      │ • 第三方系统          │
│ • 安全告警      │ • 组织变更      │ • 审批结果      │ • API 调用            │
│ • 维护通知      │ • 活动通知      │ • 抄送消息      │ • Webhook             │
└─────────────────┴─────────────────┴─────────────────┴───────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           消息模板层 (Template)                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  模板渲染引擎 (Tera) - 变量替换                                      │   │
│  │  {{title}} → "系统维护通知" | {{content}} → "今晚22:00系统升级..."    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           消息实体层 (Message Entity)                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  message_code     |  category  |  title  |  content  |  priority  |  status │
│  MSG_xxx          |  system    |  "xxx"  |  "xxx"     |  1-5       |  0/1/2  │
├─────────────────────────────────────────────────────────────────────────────┤
│  source_type      |  source_detail    |  msg_type                             │
│  "system"         |  "monitor_module" |  "system_maintenance"                  │
├─────────────────────────────────────────────────────────────────────────────┤
│  extra_data (JSON):                                                         │
│  { "target_orgs": [...], "target_roles": [...], "target_users": [...] }      │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         目标规则层 (Target Rules)                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                 │
│   │  Rule 1      │    │  Rule 2      │    │  Rule N      │                 │
│   │  ─────────   │    │  ─────────   │    │  ─────────   │                 │
│   │  target_type │    │  target_type │    │  target_type │                 │
│   │  = "org"     │ OR │  = "role"    │ OR │  = "user"    │                 │
│   │  target_scope│    │  target_scope│    │  target_scope│                 │
│   │  = {org_ids} │    │  = {roles}   │    │  = {user_ids}│                 │
│   └──────────────┘    └──────────────┘    └──────────────┘                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                        目标解析层 (Target Resolver)                          │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  输入: TargetRule[]                                                  │   │
│  │  处理: 查询数据库 (organization / user / role 表)                    │   │
│  │  输出: 去重后的 user_id 列表                                         │   │
│  │                                                                     │   │
│  │  逻辑:                                                              │   │
│  │  1. org_ids  → 查询 org 下所有用户                                   │   │
│  │  2. role_codes → 查询具有角色的所有用户                              │   │
│  │  3. user_ids → 直接使用                                              │   │
│  │  4. 合并并去重 → 最终目标用户列表                                    │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           渠道分发层 (Channel Dispatch)                      │
├─────────────────┬─────────────────┬─────────────────┬───────────────────────┤
│  日志渠道       │  WebSocket渠道  │   Email渠道     │   钉钉渠道            │
│  (LogFile)      │  (WebSocket)    │   (Email)       │   (DingTalk)          │
│  ─────────────  │  ────────────   │  ───────────    │  ─────────────        │
│  默认Stub渠道   │  实时推送       │   邮件通知      │   企业通知            │
│  记录所有分发   │  在线用户       │   离线用户      │   紧急消息            │
│  输出独立日志   │  即时送达       │   异步发送      │   工作通知            │
└─────────────────┴─────────────────┴─────────────────┴───────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           用户消息层 (User Message)                          │
├─────────────────────────────────────────────────────────────────────────────┤
│  为每个目标用户创建 user_message 记录:                                        │
│  • user_id, message_id, tenant_id, is_read, read_at, is_pinned              │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           目标用户层 (Target Users)                          │
│                                                                             │
│   User A ◄───┐                                                              │
│   User B ◄───┼── 消息分发 (支持多种渠道)                                    │
│   User C ◄───┘                                                              │
│   ...                                                                       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. 消息源 (Message Source)

### 2.1 源类型定义

```rust
pub enum MessageSource {
    System,         // 系统消息源
    Organization,   // 组织消息源
    Workflow,       // 审批任务源
    External,       // 外部集成源
}
```

### 2.2 源类型与消息类型映射

| 源类型 | source_detail 示例 | 消息类型 (msg_type) | 业务含义 |
|--------|-------------------|---------------------|----------|
| **System** | `system_monitor` | `system_announcement` | 系统公告 |
| **System** | `security_module` | `system_security` | 安全告警 |
| **System** | `ops_maintenance` | `system_maintenance` | 维护通知 |
| **Organization** | `org_1001` | `org_department` | 部门通知 |
| **Organization** | `org_1001` | `org_change` | 组织变更 |
| **Organization** | `org_1001` | `org_activity` | 活动通知 |
| **Workflow** | `wf_process_123` | `workflow_todo` | 待办任务 |
| **Workflow** | `wf_process_123` | `workflow_result` | 审批结果 |
| **Workflow** | `wf_process_123` | `workflow_cc` | 抄送消息 |

---

## 3. 消息目标 (Message Target)

### 3.1 目标规则结构

```rust
pub struct TargetRule {
    pub target_type: String,        // "org" | "role" | "user" | "condition"
    pub target_scope: Value,        // JSON 格式的目标范围
    pub filter_conditions: Option<Value>,  // 过滤条件
}
```

### 3.2 目标范围示例

```json
// 组织目标
{
  "target_type": "org",
  "target_scope": {
    "org_ids": [10, 11, 12],
    "include_children": true
  }
}

// 角色目标
{
  "target_type": "role",
  "target_scope": {
    "role_codes": ["admin", "manager", "teacher"]
  }
}

// 用户目标
{
  "target_type": "user",
  "target_scope": {
    "user_ids": [1001, 1002, 1003]
  }
}

// 复合条件目标
{
  "target_type": "condition",
  "target_scope": {
    "condition": "last_login > '2024-01-01' AND status = 'active'"
  }
}
```

### 3.3 目标解析流程

```
┌──────────────────────────────────────────────────────────────┐
│                     TargetResolver 解析流程                  │
└──────────────────────────────────────────────────────────────┘

  输入: Vec<TargetRule>
           │
           ▼
  ┌─────────────────────┐
  │  遍历每个 TargetRule │
  └─────────────────────┘
           │
           ▼
  ┌────────────────────────────────────────┐
  │  match target_type                     │
  ├────────────────────────────────────────┤
  │  "org"   → 查询 org_member 表获取用户  │
  │  "role"  → 查询 user_role 表获取用户   │
  │  "user"  → 直接使用 user_ids           │
  │  "condition" → 执行动态 SQL 查询       │
  └────────────────────────────────────────┘
           │
           ▼
  ┌─────────────────────┐
  │  收集所有 user_id    │
  │  使用 HashSet 去重   │
  └─────────────────────┘
           │
           ▼
  输出: Vec<i64> (去重后的用户ID列表)
```

---

## 4. 渠道分发架构

### 4.1 渠道 trait 设计

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
    ) -> AppResult<ChannelResult>;
}
```

### 4.2 渠道管理器

```rust
pub struct ChannelManager {
    channels: Vec<Box<dyn MessageChannel>>,
}

impl ChannelManager {
    /// 注册渠道
    pub fn register_channel(&mut self, channel: Box<dyn MessageChannel>);

    /// 获取所有支持该消息类型的渠道
    pub fn get_supported_channels(&self, msg_type: &MessageType) -> Vec<&dyn MessageChannel>;

    /// 发送消息到所有支持的渠道
    pub async fn dispatch(&self, message: &Message, target: &DispatchTarget, msg_type: &MessageType);
}
```

### 4.3 渠道实现列表

| 渠道 | 实现文件 | 用途 | 状态 |
|------|---------|------|------|
| **LogFileChannel** | `channel/log_file_channel.rs` | 默认Stub渠道，记录分发日志 | ✅ 已实现 |
| **WebSocketChannel** | (集成在 PushService) | 实时推送给在线用户 | ✅ 已实现 |
| **EmailChannel** | (待扩展) | 邮件通知 | 📋 待实现 |
| **DingTalkChannel** | (待扩展) | 钉钉工作通知 | 📋 待实现 |
| **SMSChannel** | (待扩展) | 短信通知 | 📋 待实现 |

---

## 5. 日志文件渠道详情

### 5.1 日志文件格式

**文件路径**: `logs/message-dispatch.YYYY-MM-DD.log`

**日志格式**:
```
[2024-03-23T10:30:00+08:00] MSG_abc123 | SOURCE:system | FROM:system_monitor | TYPE:system_security | ORGS:[10, 11] | ROLES:["admin", "manager"] | USERS:[1001, 1002] | CHANNELS:["log"] | STATUS:success
```

### 5.2 日志字段说明

| 字段 | 说明 | 示例 |
|------|------|------|
| `timestamp` | 分发时间 (RFC3339) | `2024-03-23T10:30:00+08:00` |
| `message_id` | 消息唯一标识 | `MSG_abc123` |
| `source_type` | 消息源类型 | `system` / `organization` / `workflow` |
| `source_detail` | 具体来源 | `system_monitor` / `org_1001` / `wf_process_123` |
| `msg_type` | 消息类型 | `system_security` / `org_change` / `workflow_todo` |
| `target_orgs` | 目标组织ID列表 | `[10, 11, 12]` |
| `target_roles` | 目标角色编码列表 | `["admin", "manager"]` |
| `target_users` | 目标用户ID列表 | `[1001, 1002, 1003]` |
| `channels` | 使用的渠道 | `["log", "websocket"]` |
| `status` | 分发状态 | `success` / `failed` |

### 5.3 日志输出示例

```
# 系统安全告警
[2024-03-23T10:30:00+08:00] MSG_550e8400-e29b-41d4-a716-446655440000 | SOURCE:system | FROM:security_module | TYPE:system_security | ORGS:[] | ROLES:["admin", "security_officer"] | USERS:[1001, 1002] | CHANNELS:["log"] | STATUS:success

# 组织变更通知
[2024-03-23T11:00:00+08:00] MSG_550e8400-e29b-41d4-a716-446655440001 | SOURCE:organization | FROM:org_1001 | TYPE:org_change | ORGS:[1001] | ROLES:["manager", "employee"] | USERS:[2001, 2002, 2003, 2004] | CHANNELS:["log"] | STATUS:success

# 审批待办任务
[2024-03-23T11:30:00+08:00] MSG_550e8400-e29b-41d4-a716-446655440002 | SOURCE:workflow | FROM:wf_process_12345 | TYPE:workflow_todo | ORGS:[] | ROLES:["approver"] | USERS:[3001] | CHANNELS:["log"] | STATUS:success
```

---

## 6. 数据流时序图

```
┌─────────┐     ┌─────────────┐     ┌──────────────┐     ┌────────────────┐     ┌─────────────┐     ┌──────────┐
│ Client  │     │ send_message│     │ TargetResolve│     │ ChannelManager │     │ LogFileChan │     │ Database │
└────┬────┘     └──────┬──────┘     └──────┬───────┘     └───────┬────────┘     └──────┬──────┘     └────┬─────┘
     │                 │                   │                     │                     │                 │
     │ POST /api/message/send             │                     │                     │                 │
     │ ─────────────────────────────────> │                     │                     │                 │
     │                 │                   │                     │                     │                 │
     │                 │ 1. Create Message │                     │                     │                 │
     │                 │ ──────────────────────────────────────────────────────────────────────────────> │
     │                 │                   │                     │                     │                 │
     │                 │ 2. Save Target Rules                    │                     │                 │
     │                 │ ──────────────────────────────────────────────────────────────────────────────> │
     │                 │                   │                     │                     │                 │
     │                 │ 3. Resolve Targets                      │                     │                 │
     │                 │ ─────────────────>│                     │                     │                 │
     │                 │                   │ 3.1 Query org/role/user                  │                 │
     │                 │                   │ ────────────────────────────────────────────────────────────> │
     │                 │                   │ <──────────────────────────────────────────────────────────── │
     │                 │ <─────────────────│                     │                     │                 │
     │                 │ (user_ids: [1,2,3])                   │                     │                 │
     │                 │                   │                     │                     │                 │
     │                 │ 4. Create User Messages                 │                     │                 │
     │                 │ ──────────────────────────────────────────────────────────────────────────────> │
     │                 │                   │                     │                     │                 │
     │                 │ 5. Dispatch to Channels                 │                     │                 │
     │                 │ ─────────────────────────────────────>│                     │                 │
     │                 │                   │                     │ 5.1 Get supported channels            │
     │                 │                   │                     │ ─────────────────>│                 │
     │                 │                   │                     │ <─────────────────│                 │
     │                 │                   │                     │                     │                 │
     │                 │                   │                     │ 5.2 Send to LogFileChannel            │
     │                 │                   │                     │ ─────────────────>│                 │
     │                 │                   │                     │                   │ 5.2.1 Write log │
     │                 │                   │                     │                   │ ────────────────│
     │                 │                   │                     │ <─────────────────│                 │
     │                 │                   │                     │                     │                 │
     │                 │ 6. Update Message Status                │                     │                 │
     │                 │ ──────────────────────────────────────────────────────────────────────────────> │
     │                 │                   │                     │                     │                 │
     │ <────────────────────────────────── │                     │                     │                 │
│     │ {message_id: 123}                │                     │                     │                 │
│     │                 │                   │                     │                     │                 │
```

---

## 7. 配置说明

### 7.1 日志目录配置

```rust
// 默认配置 - 使用项目根目录下的 logs 文件夹
let log_channel = LogFileChannel::default_with_dir();  // logs/

// 自定义配置 - 指定特定目录
let log_channel = LogFileChannel::new("/var/log/message-system");
```

### 7.2 日志轮转

- 按天自动分割日志文件
- 文件名格式: `message-dispatch.YYYY-MM-DD.log`
- 建议使用外部工具 (如 logrotate) 进行历史日志清理

---

## 8. 实现状态

| 组件 | 状态 | 说明 |
|------|------|------|
| 消息源枚举 (MessageSource) | ✅ 已完成 | `src/models/message.rs` |
| 消息类型枚举 (MessageType) | ✅ 已完成 | `src/models/message.rs` |
| 分发目标结构 (DispatchTarget) | ✅ 已完成 | `src/models/message.rs` |
| 分发日志结构 (MessageDispatchLog) | ✅ 已完成 | `src/models/message.rs` |
| 渠道 trait (MessageChannel) | ✅ 已完成 | `src/channel/mod.rs` |
| 渠道管理器 (ChannelManager) | ✅ 已完成 | `src/channel/mod.rs` |
| 日志文件渠道 (LogFileChannel) | ✅ 已完成 | `src/channel/log_file_channel.rs` |
| WebSocket 实时推送 | ✅ 已完成 | 集成在 PushService |
| Email 渠道 | 📋 待扩展 | 预留接口 |
| 钉钉渠道 | 📋 待扩展 | 预留接口 |

---

*文档版本: 1.0*
*更新日期: 2024-03-23*
