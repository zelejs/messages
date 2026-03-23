我来为你编写完整的 REST Client 格式测试用例文档。

```http
### ========================================
### 系统消息 API 测试用例集
### 基于 REST Client / HTTP Client 格式
### ========================================

### 环境变量配置
@baseUrl = http://localhost:3000/api
@tenantId = 1001
@adminToken = eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
@userToken = eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
@userId = 10001
@orgId = 5001

### ========================================
### 1. 消息模板管理
### ========================================

### 1.1 创建系统通知模板
POST {{baseUrl}}/message-templates
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "SYS_MAINTENANCE_NOTICE",
  "templateName": "系统维护通知",
  "category": "system",
  "priority": 1,
  "titleTemplate": "【紧急】系统维护通知 - {{maintenanceDate}}",
  "contentTemplate": "尊敬的用户，系统将于 {{maintenanceDate}} {{maintenanceTime}} 进行维护，预计持续 {{duration}} 小时。维护期间部分功能将不可用，请提前做好准备。",
  "jumpType": "url",
  "jumpParams": {
    "url": "/system/notice/{{noticeId}}"
  },
  "channels": ["web", "email", "dingtalk"],
  "isSystem": 1
}

### 1.2 创建业务审批模板
POST {{baseUrl}}/message-templates
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "APPROVAL_PENDING",
  "templateName": "待审批通知",
  "category": "business",
  "priority": 2,
  "titleTemplate": "您有新的{{approvalType}}待审批",
  "contentTemplate": "{{applicantName}} 提交了{{approvalType}}申请，申请单号：{{orderNo}}，请及时处理。申请原因：{{reason}}",
  "jumpType": "route",
  "jumpParams": {
    "name": "approval-detail",
    "params": ["approvalId"]
  },
  "channels": ["web", "email"]
}

### 1.3 创建告警通知模板
POST {{baseUrl}}/message-templates
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "SYSTEM_ALARM",
  "templateName": "系统告警",
  "category": "alarm",
  "priority": 1,
  "titleTemplate": "【告警】{{alarmType}} - {{alarmLevel}}",
  "contentTemplate": "告警时间：{{alarmTime}}\n告警内容：{{alarmContent}}\n影响范围：{{affectedScope}}\n建议措施：{{suggestion}}",
  "jumpType": "action",
  "jumpParams": {
    "action": "openAlarmDetail",
    "params": ["alarmId"]
  },
  "channels": ["web", "dingtalk", "sms"]
}

### 1.4 获取模板列表
GET {{baseUrl}}/message-templates?category=business&page=1&pageSize=20
Authorization: Bearer {{adminToken}}

### 1.5 获取模板详情
GET {{baseUrl}}/message-templates/SYS_MAINTENANCE_NOTICE
Authorization: Bearer {{adminToken}}

### ========================================
### 2. 消息发送 - 不同目标规则
### ========================================

### 2.1 发送给指定用户（直接指定用户ID）
POST {{baseUrl}}/messages/send
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "APPROVAL_PENDING",
  "targetRules": [
    {
      "targetType": "user",
      "targetScope": {
        "userIds": [10001, 10002, 10003]
      }
    }
  ],
  "variables": {
    "approvalType": "请假申请",
    "applicantName": "张三",
    "orderNo": "QJ202402050001",
    "reason": "个人事务",
    "approvalId": "12345"
  },
  "sendType": 1
}

### 2.2 发送给指定组织（包含子组织）
POST {{baseUrl}}/messages/send
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "SYS_MAINTENANCE_NOTICE",
  "targetRules": [
    {
      "targetType": "org",
      "targetScope": {
        "orgIds": [5001, 5002],
        "includeChildren": true
      }
    }
  ],
  "variables": {
    "maintenanceDate": "2026-02-10",
    "maintenanceTime": "02:00-06:00",
    "duration": "4",
    "noticeId": "67890"
  },
  "sendType": 1
}

### 2.3 发送给指定组织（不包含子组织）
POST {{baseUrl}}/messages/send
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "APPROVAL_PENDING",
  "targetRules": [
    {
      "targetType": "org",
      "targetScope": {
        "orgIds": [5001],
        "includeChildren": false
      }
    }
  ],
  "variables": {
    "approvalType": "预算申请",
    "applicantName": "李四",
    "orderNo": "YS202402050001",
    "reason": "部门年度预算",
    "approvalId": "12346"
  },
  "sendType": 1
}

### 2.4 发送给指定角色
POST {{baseUrl}}/messages/send
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "SYSTEM_ALARM",
  "targetRules": [
    {
      "targetType": "role",
      "targetScope": {
        "roleCodes": ["admin", "system_manager"]
      }
    }
  ],
  "variables": {
    "alarmType": "服务器负载过高",
    "alarmLevel": "严重",
    "alarmTime": "2026-02-05 14:30:00",
    "alarmContent": "服务器CPU使用率达到95%",
    "affectedScope": "所有用户",
    "suggestion": "立即检查服务器状态",
    "alarmId": "ALM20260205001"
  },
  "sendType": 1
}

### 2.5 自定义条件发送（复杂SQL条件）
POST {{baseUrl}}/messages/send
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "SYS_MAINTENANCE_NOTICE",
  "targetRules": [
    {
      "targetType": "custom",
      "targetScope": {
        "condition": "department = '技术部' AND level >= 3 AND status = 1"
      }
    }
  ],
  "variables": {
    "maintenanceDate": "2026-02-15",
    "maintenanceTime": "00:00-04:00",
    "duration": "4",
    "noticeId": "67891"
  },
  "sendType": 1
}

### 2.6 混合多种目标规则（用户 + 组织 + 角色）
POST {{baseUrl}}/messages/send
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "APPROVAL_PENDING",
  "targetRules": [
    {
      "targetType": "user",
      "targetScope": {
        "userIds": [10001]
      }
    },
    {
      "targetType": "org",
      "targetScope": {
        "orgIds": [5003],
        "includeChildren": true
      }
    },
    {
      "targetType": "role",
      "targetScope": {
        "roleCodes": ["manager"]
      }
    }
  ],
  "variables": {
    "approvalType": "采购申请",
    "applicantName": "王五",
    "orderNo": "CG202402050001",
    "reason": "办公设备采购",
    "approvalId": "12347"
  },
  "sendType": 1
}

### 2.7 带筛选条件的发送（组织 + 筛选条件）
POST {{baseUrl}}/messages/send
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "SYS_MAINTENANCE_NOTICE",
  "targetRules": [
    {
      "targetType": "org",
      "targetScope": {
        "orgIds": [5001],
        "includeChildren": true
      },
      "filterConditions": {
        "field": "last_login_at",
        "operator": ">=",
        "value": "2026-01-01"
      }
    }
  ],
  "variables": {
    "maintenanceDate": "2026-02-20",
    "maintenanceTime": "03:00-05:00",
    "duration": "2",
    "noticeId": "67892"
  },
  "sendType": 1
}

### 2.8 定时发送消息
POST {{baseUrl}}/messages/send
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "SYS_MAINTENANCE_NOTICE",
  "targetRules": [
    {
      "targetType": "org",
      "targetScope": {
        "orgIds": [5001],
        "includeChildren": true
      }
    }
  ],
  "variables": {
    "maintenanceDate": "2026-02-25",
    "maintenanceTime": "02:00-06:00",
    "duration": "4",
    "noticeId": "67893"
  },
  "sendType": 2,
  "scheduledAt": "2026-02-24T18:00:00Z"
}

### ========================================
### 3. 用户消息查询
### ========================================

### 3.1 获取所有未读消息
GET {{baseUrl}}/messages?isRead=0&page=1&pageSize=20
Authorization: Bearer {{userToken}}

### 3.2 获取指定分类的消息（业务类）
GET {{baseUrl}}/messages?category=business&page=1&pageSize=20
Authorization: Bearer {{userToken}}

### 3.3 获取指定分类的消息（系统类）
GET {{baseUrl}}/messages?category=system&page=1&pageSize=20
Authorization: Bearer {{userToken}}

### 3.4 获取指定分类的消息（告警类）
GET {{baseUrl}}/messages?category=alarm&page=1&pageSize=20
Authorization: Bearer {{userToken}}

### 3.5 获取已读消息
GET {{baseUrl}}/messages?isRead=1&page=1&pageSize=20
Authorization: Bearer {{userToken}}

### 3.6 获取指定优先级的消息（紧急）
GET {{baseUrl}}/messages?priority=1&page=1&pageSize=20
Authorization: Bearer {{userToken}}

### 3.7 获取指定时间范围的消息
GET {{baseUrl}}/messages?startDate=2026-02-01&endDate=2026-02-05&page=1&pageSize=20
Authorization: Bearer {{userToken}}

### 3.8 获取未读消息统计（按分类）
GET {{baseUrl}}/messages/unread-stats
Authorization: Bearer {{userToken}}

### ========================================
### 4. 消息操作
### ========================================

### 4.1 标记单条消息为已读
POST {{baseUrl}}/messages/123456/read
Authorization: Bearer {{userToken}}

### 4.2 批量标记消息为已读
POST {{baseUrl}}/messages/batch-read
Content-Type: application/json
Authorization: Bearer {{userToken}}

{
  "messageIds": [123456, 123457, 123458]
}

### 4.3 标记分类下所有消息为已读
POST {{baseUrl}}/messages/read-by-category
Content-Type: application/json
Authorization: Bearer {{userToken}}

{
  "category": "business"
}

### 4.4 标记所有消息为已读
POST {{baseUrl}}/messages/read-all
Authorization: Bearer {{userToken}}

### 4.5 删除单条消息
DELETE {{baseUrl}}/messages/123456
Authorization: Bearer {{userToken}}

### 4.6 批量删除消息
POST {{baseUrl}}/messages/batch-delete
Content-Type: application/json
Authorization: Bearer {{userToken}}

{
  "messageIds": [123456, 123457, 123458]
}

### 4.7 置顶消息
POST {{baseUrl}}/messages/123456/pin
Authorization: Bearer {{userToken}}

### 4.8 取消置顶
DELETE {{baseUrl}}/messages/123456/pin
Authorization: Bearer {{userToken}}

### 4.9 获取消息详情
GET {{baseUrl}}/messages/123456
Authorization: Bearer {{userToken}}

### ========================================
### 5. 用户消息偏好设置
### ========================================

### 5.1 获取当前用户的消息偏好设置
GET {{baseUrl}}/message-settings
Authorization: Bearer {{userToken}}

### 5.2 更新消息偏好设置（开启邮件通知）
PUT {{baseUrl}}/message-settings
Content-Type: application/json
Authorization: Bearer {{userToken}}

{
  "settings": [
    {
      "category": "business",
      "webEnabled": true,
      "emailEnabled": true,
      "dingtalkEnabled": false
    },
    {
      "category": "system",
      "webEnabled": true,
      "emailEnabled": true,
      "dingtalkEnabled": true
    },
    {
      "category": "alarm",
      "webEnabled": true,
      "emailEnabled": true,
      "dingtalkEnabled": true
    }
  ]
}

### 5.3 设置免打扰时段
PUT {{baseUrl}}/message-settings/dnd
Content-Type: application/json
Authorization: Bearer {{userToken}}

{
  "doNotDisturb": true,
  "dndStartTime": "22:00:00",
  "dndEndTime": "08:00:00"
}

### 5.4 关闭免打扰
PUT {{baseUrl}}/message-settings/dnd
Content-Type: application/json
Authorization: Bearer {{userToken}}

{
  "doNotDisturb": false
}

### 5.5 批量更新分类推送渠道
PUT {{baseUrl}}/message-settings/channels
Content-Type: application/json
Authorization: Bearer {{userToken}}

{
  "category": "business",
  "webEnabled": true,
  "emailEnabled": false,
  "dingtalkEnabled": true
}

### ========================================
### 6. 组织和用户关系查询（用于目标解析测试）
### ========================================

### 6.1 获取组织树结构
GET {{baseUrl}}/organizations/tree?tenantId={{tenantId}}
Authorization: Bearer {{adminToken}}

### 6.2 获取指定组织的所有用户（不含子组织）
GET {{baseUrl}}/organizations/{{orgId}}/users?includeChildren=false
Authorization: Bearer {{adminToken}}

### 6.3 获取指定组织的所有用户（含子组织）
GET {{baseUrl}}/organizations/{{orgId}}/users?includeChildren=true
Authorization: Bearer {{adminToken}}

### 6.4 获取指定角色的所有用户
GET {{baseUrl}}/roles/admin/users
Authorization: Bearer {{adminToken}}

### 6.5 根据自定义条件查询用户
POST {{baseUrl}}/users/query
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "condition": "department = '技术部' AND level >= 3"
}

### ========================================
### 7. 消息管理（管理员功能）
### ========================================

### 7.1 查询租户下所有消息（管理员）
GET {{baseUrl}}/admin/messages?tenantId={{tenantId}}&page=1&pageSize=20
Authorization: Bearer {{adminToken}}

### 7.2 查询指定消息的发送详情
GET {{baseUrl}}/admin/messages/123456/details
Authorization: Bearer {{adminToken}}

### 7.3 查询消息推送日志
GET {{baseUrl}}/admin/messages/123456/push-logs
Authorization: Bearer {{adminToken}}

### 7.4 撤回消息（未读消息）
POST {{baseUrl}}/admin/messages/123456/revoke
Authorization: Bearer {{adminToken}}

### 7.5 取消定时消息
POST {{baseUrl}}/admin/messages/123456/cancel
Authorization: Bearer {{adminToken}}

### 7.6 重新推送失败的消息
POST {{baseUrl}}/admin/messages/123456/retry
Authorization: Bearer {{adminToken}}

### 7.7 获取消息发送统计
GET {{baseUrl}}/admin/messages/stats?startDate=2026-02-01&endDate=2026-02-05
Authorization: Bearer {{adminToken}}

### 7.8 导出消息记录
GET {{baseUrl}}/admin/messages/export?startDate=2026-02-01&endDate=2026-02-05&format=csv
Authorization: Bearer {{adminToken}}

### ========================================
### 8. 复杂业务场景测试
### ========================================

### 8.1 场景：部门会议通知（组织 + 筛选在职员工）
POST {{baseUrl}}/messages/send
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "MEETING_NOTICE",
  "targetRules": [
    {
      "targetType": "org",
      "targetScope": {
        "orgIds": [5001],
        "includeChildren": false
      },
      "filterConditions": {
        "field": "status",
        "operator": "=",
        "value": "1"
      }
    }
  ],
  "variables": {
    "meetingTitle": "2026年Q1部门总结会",
    "meetingTime": "2026-02-10 14:00",
    "meetingLocation": "会议室A",
    "meetingId": "MT20260210001"
  },
  "sendType": 1
}

### 8.2 场景：紧急通知所有在线管理员
POST {{baseUrl}}/messages/send
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "URGENT_NOTICE",
  "targetRules": [
    {
      "targetType": "role",
      "targetScope": {
        "roleCodes": ["admin", "manager"]
      },
      "filterConditions": {
        "field": "last_active_at",
        "operator": ">=",
        "value": "2026-02-05T00:00:00Z"
      }
    }
  ],
  "variables": {
    "urgentContent": "系统检测到异常流量",
    "actionRequired": "请立即登录后台查看",
    "noticeId": "URG20260205001"
  },
  "sendType": 1
}

### 8.3 场景：跨组织协作通知（多个组织的特定角色）
POST {{baseUrl}}/messages/send
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "COLLABORATION_NOTICE",
  "targetRules": [
    {
      "targetType": "org",
      "targetScope": {
        "orgIds": [5001, 5002, 5003],
        "includeChildren": false
      }
    },
    {
      "targetType": "role",
      "targetScope": {
        "roleCodes": ["project_manager"]
      }
    }
  ],
  "variables": {
    "projectName": "跨部门协作项目X",
    "kickoffTime": "2026-02-08 10:00",
    "projectId": "PRJ20260205001"
  },
  "sendType": 1
}

### 8.4 场景：VIP用户专属通知
POST {{baseUrl}}/messages/send
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "VIP_EXCLUSIVE",
  "targetRules": [
    {
      "targetType": "custom",
      "targetScope": {
        "condition": "user_level = 'VIP' AND status = 1"
      }
    }
  ],
  "variables": {
    "activityName": "VIP客户专属活动",
    "activityDate": "2026-02-15",
    "benefitDetails": "享受8折优惠",
    "activityId": "ACT20260205001"
  },
  "sendType": 1
}

### 8.5 场景：新员工入职欢迎（最近7天入职）
POST {{baseUrl}}/messages/send
Content-Type: application/json
Authorization: Bearer {{adminToken}}

{
  "templateCode": "WELCOME_NEWBIE",
  "targetRules": [
    {
      "targetType": "custom",
      "targetScope": {
        "condition": "created_at >= DATE_SUB(NOW(), INTERVAL 7 DAY)"
      }
    }
  ],
  "variables": {
    "companyName": "某某科技有限公司",
    "hrContact": "hr@example.com",
    "welcomeGuide": "https://wiki.example.com/newbie-guide"
  },
  "sendType": 1
}

### ========================================
### 9. 消息聚合和统计
### ========================================

### 9.1 获取未读消息数（按分类聚合）
GET {{baseUrl}}/messages/unread-count
Authorization: Bearer {{userToken}}

### 9.2 获取最近的消息（最近7天）
GET {{baseUrl}}/messages/recent?days=7
Authorization: Bearer {{userToken}}

### 9.3 获取消息接收趋势（管理员）
GET {{baseUrl}}/admin/messages/trend?startDate=2026-01-01&endDate=2026-02-05&groupBy=day
Authorization: Bearer {{adminToken}}

### 9.4 获取各分类消息占比（管理员）
GET {{baseUrl}}/admin/messages/category-stats?startDate=2026-02-01&endDate=2026-02-05
Authorization: Bearer {{adminToken}}

### 9.5 获取推送渠道效果统计（管理员）
GET {{baseUrl}}/admin/messages/channel-stats?startDate=2026-02-01&endDate=2026-02-05
Authorization: Bearer {{adminToken}}

### ========================================
### 10. WebSocket 连接测试（参考用例）
### ========================================

### 10.1 建立 WebSocket 连接（伪代码，仅供参考）
# CONNECT ws://localhost:3000/tenant-{{tenantId}}
# Headers:
#   Authorization: Bearer {{userToken}}
# Params:
#   userId: {{userId}}

### 10.2 监听新消息事件（伪代码）
# ON EVENT: new_message
# Callback: (message) => {
#   console.log('收到新消息:', message);
#   // 更新UI
# }

### 10.3 断开连接（伪代码）
# DISCONNECT

### ========================================
### 测试用例说明
### ========================================

# 1. 目标规则测试覆盖：
#    - 用户维度（user）
#    - 组织维度（org，含/不含子组织）
#    - 角色维度（role）
#    - 自定义条件（custom）
#    - 混合规则（多种维度组合）
#    - 带筛选条件的规则

# 2. 消息发送场景：
#    - 立即发送
#    - 定时发送
#    - 多租户隔离
#    - 不同优先级
#    - 不同分类

# 3. 用户操作：
#    - 查询（分页、筛选、分类）
#    - 已读/未读标记
#    - 删除
#    - 置顶
#    - 偏好设置

# 4. 管理功能：
#    - 消息管理
#    - 统计分析
#    - 推送日志
#    - 消息撤回

# 5. 复杂业务场景：
#    - 跨组织协作
#    - 条件筛选
#    - 时间范围
#    - 用户等级
```

这份测试用例文档涵盖了:

1. **完整的业务流程**: 从模板创建到消息发送、接收、处理
2. **多维度目标规则**: user/org/role/custom 及其组合
3. **核心功能**: 已读/未读、删除、置顶、偏好设置
4. **管理功能**: 统计、日志、撤回等
5. **实际业务场景**: 会议通知、紧急通知、跨组织协作等

你可以直接在 VS Code 安装 REST Client 插件，或使用 IntelliJ IDEA 的 HTTP Client 来运行这些测试用例。需要我补充其他特定场景的测试吗?
