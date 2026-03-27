# Message Module - 任务进度跟踪

## 当前任务

### ✅ 已完成

1. **表重命名: `t_sys_user_messages` -> `t_sys_message_users`**
   - DDL 文件: `ddl/messages-schema.sql`
   - Repository 文件: `src/repositories/message_repository.rs`
   - 所有 SQL 查询已更新

2. **重试机制 (Retry Mechanism)**
   - 模型: `src/models/retry.rs`
     - `MessageRetryRecord` - 重试记录
     - `RetryAttempt` - 单次重试尝试
     - `RetryQuery` - 重试查询参数
   - Repository: `src/repositories/retry_repository.rs`
   - Service: `src/services/retry_service.rs`
     - `RetryService` - 重试逻辑管理
     - `DLQService` - 死信队列管理
   - 配置: `src/config.rs`
     - `RetryConfig` - 重试配置结构
   - 环境变量: `.env.example`
     - `RETRY_ENABLED` - 启用重试
     - `RETRY_MAX_RETRIES` - 最大重试次数
     - `RETRY_INTERVALS` - 重试间隔(逗号分隔)

3. **死信队列 (Dead Letter Queue)**
   - DDL 新增表:
     - `t_sys_message_retry_records` - 重试记录表
     - `t_sys_message_dead_letters` - 死信队列表
   - Admin API 新增:
     - `GET /api/adm/message/dlq/list` - 列死信消息
     - `GET /api/adm/message/dlq/stats` - 死信统计
     - `POST /api/adm/message/dlq/:id/retry` - 重试死信
     - `POST /api/adm/message/dlq/:id/abandon` - 放弃死信
     - `DELETE /api/adm/message/dlq/:id` - 删除死信

4. **重试机制集成到 PushService** ✅
   - `PushService` 集成 `RetryService` 和 `DLQService`
   - 推送失败时自动创建重试记录
   - 重试工作线程每30秒轮询待重试消息
   - 重试失败后自动移入死信队列

## 实现详情

### 重试配置示例

```bash
# .env
RETRY_ENABLED=true
RETRY_MAX_RETRIES=3
RETRY_INTERVALS=60,300,900  # 1分钟, 5分钟, 15分钟
```

### 重试记录表结构

```sql
CREATE TABLE t_sys_message_retry_records (
    id BIGSERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    channel VARCHAR(20) NOT NULL,
    retry_count SMALLINT DEFAULT 0,
    max_retries SMALLINT DEFAULT 3,
    retry_intervals JSONB DEFAULT '[60, 300, 900]',
    next_retry_at TIMESTAMPTZ,
    last_error TEXT,
    retry_history JSONB DEFAULT '[]',
    status SMALLINT DEFAULT 0,  -- 0:待重试 1:成功 2:进入死信
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);
```

### 死信队列表结构

```sql
CREATE TABLE t_sys_message_dead_letters (
    id BIGSERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    channel VARCHAR(20) NOT NULL,
    failed_reason TEXT,
    retry_history JSONB,
    status SMALLINT DEFAULT 0,  -- 0:待处理 1:已重试 2:已放弃
    retried_at TIMESTAMPTZ,
    retried_success SMALLINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);
```

### 重试流程

```
消息推送失败
    │
    ▼
创建重试记录 (retry_count=0, next_retry_at=now+60s)
    │
    ▼
等待定时任务轮询 (每30秒)
    │
    ▼
执行重试 ──成功──> 标记成功, 删除重试记录
    │
    └──失败──> retry_count++, 计算 next_retry_at
                  │
                  ▼
            retry_count < max_retries?
                  │
         是 ─────┴───── 否
         │                │
         ▼                ▼
    继续等待重试      移入死信队列
```

### PushService 集成点

1. **推送失败时创建重试记录** (`push_service.rs:220-237`)
   ```rust
   Err(err) => {
       // 推送失败 - 创建重试记录
       let error_msg = format!("{:?}", err);
       if self.retry_service.should_retry(message.id, user_id, channel.name()).await? {
           self.retry_service.create_retry(message.id, user_id, channel.name(), &error_msg).await?;
       }
   }
   ```

2. **重试工作线程** (`main.rs:133-202`)
   - 每30秒轮询 `t_sys_message_retry_records` 表
   - 执行 `PushService::process_retry()` 重试推送
   - 成功则标记完成，失败则更新重试次数或移入DLQ

## API 文档

### DLQ Admin APIs

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/adm/message/dlq/list?status=0&page=1&page_size=20` | 列死信消息 |
| GET | `/api/adm/message/dlq/stats` | 死信统计 |
| POST | `/api/adm/message/dlq/:id/retry` | 管理员重试 |
| POST | `/api/adm/message/dlq/:id/abandon` | 放弃消息 |
| DELETE | `/api/adm/message/dlq/:id` | 删除消息 |

### 重试统计

```json
GET /api/adm/message/dlq/stats
{
  "pending": 5,      // 待处理
  "retried": 10,     // 已重试
  "abandoned": 2,    // 已放弃
  "total": 17        // 总计
}
```

## 代码编译状态

✅ **编译成功** (2026-03-24)
- 无错误
- 12个警告 (未使用变量等)

## 下一步计划

- [ ] 添加幂等性支持 (idempotency_key)
- [ ] 完善推送日志查询 API
- [ ] 添加消息统计报表
- [ ] 实现消息搜索功能
