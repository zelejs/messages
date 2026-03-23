# API Path Mapping

## 接口规范

- **业务前缀**: `/api/message/` - 面向普通用户的消息接口
- **后台管理前缀**: `/api/adm/message/` - 面向管理员的管理接口

## 路径映射表

### 消息模板接口 (Message Templates)

| 原路径 | 新路径 | 方法 | 说明 |
|--------|--------|------|------|
| `/api/message-templates` | `/api/message/templates` | POST | 创建模板 |
| `/api/message-templates` | `/api/message/templates` | GET | 列出模板 |
| `/api/message-templates/code/:code` | `/api/message/templates/code/:code` | GET | 按code获取 |
| `/api/message-templates/:id` | `/api/message/templates/:id` | GET | 按ID获取 |
| `/api/message-templates/:id` | `/api/message/templates/:id` | PUT | 更新模板 |
| `/api/message-templates/:id` | `/api/message/templates/:id` | DELETE | 删除模板 |

### 消息发送接口 (Message Sending)

| 原路径 | 新路径 | 方法 | 说明 |
|--------|--------|------|------|
| `/api/messages/send` | `/api/message/send` | POST | 发送消息 |

### 用户消息接口 (User Messages)

| 原路径 | 新路径 | 方法 | 说明 |
|--------|--------|------|------|
| `/api/messages` | `/api/message/list` | GET | 获取消息列表 |
| `/api/messages/:id` | `/api/message/:id` | GET | 获取消息详情 |
| `/api/messages/:id/read` | `/api/message/:id/read` | POST | 标记已读 |
| `/api/messages/batch-read` | `/api/message/batch-read` | POST | 批量标记已读 |
| `/api/messages/read-by-category` | `/api/message/read-by-category` | POST | 按分类标记已读 |
| `/api/messages/read-all` | `/api/message/read-all` | POST | 全部标记已读 |
| `/api/messages/:id` | `/api/message/:id` | DELETE | 删除消息 |
| `/api/messages/batch-delete` | `/api/message/batch-delete` | POST | 批量删除 |
| `/api/messages/:id/pin` | `/api/message/:id/pin` | POST | 置顶消息 |
| `/api/messages/:id/pin` | `/api/message/:id/pin` | DELETE | 取消置顶 |
| `/api/messages/unread-count` | `/api/message/unread-count` | GET | 未读数量 |
| `/api/messages/unread-stats` | `/api/message/unread-stats` | GET | 未读统计 |

### 用户设置接口 (User Settings)

| 原路径 | 新路径 | 方法 | 说明 |
|--------|--------|------|------|
| `/api/message-settings` | `/api/message/settings` | GET | 获取设置 |
| `/api/message-settings` | `/api/message/settings` | PUT | 更新设置 |
| `/api/message-settings/dnd` | `/api/message/settings/dnd` | PUT | 更新免打扰 |
| `/api/message-settings/channels` | `/api/message/settings/channels` | PUT | 更新渠道设置 |

### 组织接口 (Organization)

| 原路径 | 新路径 | 方法 | 说明 |
|--------|--------|------|------|
| `/api/organizations/tree` | `/api/message/org-tree` | GET | 获取组织树 |
| `/api/organizations/:id/users` | `/api/message/org-users/:id` | GET | 获取组织用户 |

### 管理后台接口 (Admin)

| 原路径 | 新路径 | 方法 | 说明 |
|--------|--------|------|------|
| `/api/admin/messages` | `/api/adm/message/list` | GET | 获取所有消息 |
| `/api/admin/messages/:id/details` | `/api/adm/message/:id/details` | GET | 获取消息详情 |
| `/api/admin/messages/:id/push-logs` | `/api/adm/message/:id/push-logs` | GET | 获取推送日志 |
| `/api/admin/messages/:id/revoke` | `/api/adm/message/:id/revoke` | POST | 撤回消息 |
| `/api/admin/messages/:id/cancel` | `/api/adm/message/:id/cancel` | POST | 取消定时消息 |
| `/api/admin/messages/:id/retry` | `/api/adm/message/:id/retry` | POST | 重试失败消息 |
| `/api/admin/messages/stats` | `/api/adm/message/stats` | GET | 获取统计信息 |

## 文件修改清单

1. **src/main.rs** - 路由定义更新
2. **tests/message-module-tests.http** - 测试用例路径更新
3. **CLAUDE.md** - API文档路径更新

## 注意事项

1. 所有业务接口统一使用 `/api/message/` 前缀
2. 所有管理接口统一使用 `/api/adm/message/` 前缀
3. WebSocket 连接路径保持不变: `/ws/:tenant_id`
4. 健康检查路径保持不变: `/health`
