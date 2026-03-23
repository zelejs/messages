# Message Module - Claude Code Instructions

## Project Overview

A **multi-tenant SaaS message system** built with Rust, designed for educational platforms. Provides message templating, targeted message delivery, real-time WebSocket notifications, and multi-channel push (email, SMS, DingTalk, in-app).

**Technology Stack:**
- Rust + Axum (async web framework)
- PostgreSQL (SQLx for database access)
- Redis (caching, online status)
- RabbitMQ (async message queue)
- WebSocket (real-time notifications)

**Current Status**: Core implementation complete, minor compilation issues remain (see IMPLEMENTATION_STATUS.md)

---

## User Intent Detection & Skill Triggers

When users request specific tasks, trigger the corresponding skill from `../saas-skills/saas-e2e-design`:

| User Request Pattern | Trigger Skill | Purpose |
|---------------------|---------------|---------|
| "设计API", "生成接口文档", "API设计" | `http-design-skill` | Generate `.http` REST Client test cases |
| "生成数据库脚本", "DDL设计", "数据库Schema" | `ddl-design-skill` | Generate database DDL scripts |
| "实现EAV API", "Rust实现", "API全流程" | `api-task-skill` | EAV-Rust API implementation workflow |
| "需求分析", "业务建模", "领域划分" | `saas-e2e-design` | Full 8-phase requirement analysis |
| "生成测试用例", ".http文件", "REST测试" | `http-design-skill` | API test case generation |

**External Skills Location**: `../saas-skills/saas-e2e-design/`

---

## Project Structure

```
message-module/
├── src/
│   ├── main.rs                    # Application entry
│   ├── config.rs                  # Environment configuration
│   ├── error.rs                   # Unified error handling
│   ├── models/                    # Data models (Tenant, User, Message, Template...)
│   ├── handlers/                  # API endpoints
│   ├── services/                  # Business logic
│   ├── repositories/              # Data access layer
│   ├── websocket/                 # Real-time communication
│   ├── queue/                     # RabbitMQ producer/consumer
│   ├── cache/                     # Redis caching layer
│   ├── middleware/                # JWT authentication
│   └── utils/                     # Pagination, JWT utilities
├── migrations/                    # 13 SQL migration files
├── Cargo.toml                     # Dependencies (rustls for TLS)
├── .env.example                   # Environment template
├── SKILLS_RULE.md                 # Skills catalog and triggers
└── IMPLEMENTATION_STATUS.md       # Current status
```

---

## API Endpoints

### Message Templates (Business Prefix: /api/message/)
- `POST /api/message/templates` - Create template
- `GET /api/message/templates` - List templates
- `GET /api/message/templates/code/:code` - Get by code
- `GET /api/message/templates/:id` - Get by ID
- `PUT /api/message/templates/:id` - Update template
- `DELETE /api/message/templates/:id` - Delete template

### Messages (Business Prefix: /api/message/)
- `POST /api/message/send` - Send message
- `GET /api/message/list` - List user messages
- `GET /api/message/:id` - Get message detail
- `POST /api/message/:id/read` - Mark as read
- `POST /api/message/batch-read` - Batch mark as read
- `POST /api/message/read-by-category` - Mark category as read
- `POST /api/message/read-all` - Mark all as read
- `DELETE /api/message/:id` - Delete message
- `POST /api/message/batch-delete` - Batch delete
- `POST /api/message/:id/pin` - Pin message
- `DELETE /api/message/:id/pin` - Unpin message
- `GET /api/message/unread-count` - Get unread count
- `GET /api/message/unread-stats` - Get unread stats

### Settings (Business Prefix: /api/message/)
- `GET /api/message/settings` - Get user settings
- `PUT /api/message/settings` - Update settings
- `PUT /api/message/settings/dnd` - Update DND mode
- `PUT /api/message/settings/channels` - Update channel preferences

### Organization (Business Prefix: /api/message/)
- `GET /api/message/org-tree` - Get organization tree
- `GET /api/message/org-users/:id` - Get organization users

### Admin (Admin Prefix: /api/adm/message/)
- `GET /api/adm/message/list` - List all messages
- `GET /api/adm/message/:id/details` - Get message details
- `GET /api/adm/message/:id/push-logs` - Get push logs
- `POST /api/adm/message/:id/revoke` - Revoke message
- `POST /api/adm/message/:id/cancel` - Cancel scheduled message
- `POST /api/adm/message/:id/retry` - Retry failed message
- `GET /api/adm/message/stats` - Get statistics

### WebSocket
- `WS /ws/:tenant_id` - Real-time message connection

---

## Core Domains

| Domain | Description | Key Entities |
|--------|-------------|--------------|
| **Message Templates** | Reusable message templates with variable substitution | `message_templates` |
| **Messages** | Core message entity with delivery tracking | `messages`, `user_messages` |
| **Target Rules** | Dynamic recipient resolution based on conditions | `message_target_rules` |
| **Push Logs** | Delivery status tracking per channel | `message_push_logs` |
| **User Settings** | Notification preferences and DND mode | `user_message_settings` |

---

## Development Guidelines

### Code Style
- Use async/await with Tokio runtime
- Error handling via `anyhow::Result` and custom `AppError`
- Database queries with SQLx (dynamic queries for flexibility)
- All handlers return `axum::response::Result`

### Database
- Multi-tenant: All tables include `tenant_id`
- Soft deletes: `deleted_at` column
- Audit fields: `created_at`, `updated_at` automatic

### Security
- JWT-based authentication via middleware
- Tenant isolation enforced at repository layer
- No sensitive data in logs

---

## Known Issues

1. **FromRow derivation**: Some custom structs need manual implementation
2. **JWT extractor lifetime**: Axum extractor pattern needs adjustment
3. **Database migrations**: Need manual execution or migration tool

See `IMPLEMENTATION_STATUS.md` for details.

---

## Running the Service

```bash
# Configure environment
cp .env.example .env
# Edit .env with your database/Redis/RabbitMQ credentials

# Run migrations (manual or via tool)
psql -h localhost -U postgres -d message_system -f migrations/001_create_tenants.sql
# ... repeat for all migrations

# Start service
cargo run

# Build release
cargo build --release
```

---

## Skills Workflow

When triggering skills, follow this pattern:

1. **User Request** → Detect intent from patterns above
2. **Load Skill** → Read from `../saas-skills/saas-e2e-design/{skill-name}/SKILL.md`
3. **Execute** → Follow skill's phase/steps
4. **Output** → Generate files in appropriate directories

Example:
- User: "生成消息模块的API测试用例"
- Intent: API test generation → `http-design-skill`
- Execute: Generate `tests/message-module-tests.http`

---

## Dependencies Note

- **TLS**: Using `rustls` instead of OpenSSL (no pkg-config required on Windows)
- **Database**: SQLx with dynamic queries (no compile-time DB connection needed)
- **Redis**: `rustls` feature enabled for TLS connections

---

*Last Updated: 2026-03-22*
*Skills Source: ../saas-skills/saas-e2e-design (renamed from saas-e2e-requirements)*
