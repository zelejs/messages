-- ============================================================================
-- Message System - Complete Database Schema
-- Multi-tenant SaaS Message Module for Educational Platforms
-- ============================================================================
-- This file contains the complete database schema for the message system.
-- It includes all tables, indexes, triggers, and comments.
--
-- Usage:
--   psql -h localhost -U postgres -d message_system -f ddl/messages-schema.sql
--
-- Database: message_system (PostgreSQL 14+)
-- Created: 2026-03-23
-- ============================================================================

-- ============================================================================
-- Extension: pg_trgm for text search (required by organizations.path index)
-- ============================================================================
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- ============================================================================
-- Table: tenants (租户表)
-- ============================================================================
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

-- ============================================================================
-- Table: organizations (组织架构表)
-- ============================================================================
CREATE TABLE organizations (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    parent_id BIGINT DEFAULT 0,
    org_code VARCHAR(50) NOT NULL,
    org_name VARCHAR(100) NOT NULL,
    org_type VARCHAR(20),
    level INT DEFAULT 1,
    path VARCHAR(500),
    status SMALLINT DEFAULT 1,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_organizations_tenant ON organizations(tenant_id);
CREATE INDEX idx_organizations_parent ON organizations(parent_id);
CREATE INDEX idx_organizations_path ON organizations USING gin(path gin_trgm_ops);
CREATE UNIQUE INDEX idx_organizations_code ON organizations(tenant_id, org_code);

COMMENT ON TABLE organizations IS '组织架构表';
COMMENT ON COLUMN organizations.org_type IS '组织类型:department/team/group';
COMMENT ON COLUMN organizations.path IS '组织路径 如:1/2/5';

-- ============================================================================
-- Table: users (用户表)
-- ============================================================================
CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    username VARCHAR(50) UNIQUE NOT NULL,
    email VARCHAR(100),
    phone VARCHAR(20),
    status SMALLINT DEFAULT 1,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    last_login_at TIMESTAMPTZ
);

CREATE INDEX idx_users_tenant ON users(tenant_id);
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_status ON users(status);

COMMENT ON TABLE users IS '用户表';

-- ============================================================================
-- Table: user_organizations (用户组织关系表)
-- ============================================================================
CREATE TABLE user_organizations (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    organization_id BIGINT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    is_primary SMALLINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, organization_id)
);

CREATE INDEX idx_user_organizations_org ON user_organizations(organization_id);

COMMENT ON TABLE user_organizations IS '用户组织关系表';
COMMENT ON COLUMN user_organizations.is_primary IS '是否主组织';

-- ============================================================================
-- Table: roles (角色表)
-- ============================================================================
CREATE TABLE roles (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    role_code VARCHAR(50) NOT NULL,
    role_name VARCHAR(100) NOT NULL,
    status SMALLINT DEFAULT 1,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(tenant_id, role_code)
);

CREATE INDEX idx_roles_tenant ON roles(tenant_id);

COMMENT ON TABLE roles IS '角色表';

-- ============================================================================
-- Table: user_roles (用户角色关系表)
-- ============================================================================
CREATE TABLE user_roles (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id BIGINT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, role_id)
);

CREATE INDEX idx_user_roles_role ON user_roles(role_id);

COMMENT ON TABLE user_roles IS '用户角色关系表';

-- ============================================================================
-- Table: message_templates (消息模板表)
-- ============================================================================
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

-- ============================================================================
-- Table: messages (消息表)
-- ============================================================================
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
    sender_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    sender_type VARCHAR(20) DEFAULT 'user',
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
COMMENT ON COLUMN messages.status IS '0:待发送 1:已发送 2:已取消 3:失败';

-- ============================================================================
-- Table: message_target_rules (消息接收规则表)
-- ============================================================================
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

-- ============================================================================
-- Table: user_messages (用户消息表)
-- ============================================================================
CREATE TABLE user_messages (
    id BIGSERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
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

-- ============================================================================
-- Table: user_message_settings (用户消息配置表)
-- ============================================================================
CREATE TABLE user_message_settings (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
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

-- ============================================================================
-- Table: message_push_logs (消息推送记录表)
-- ============================================================================
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

-- ============================================================================
-- Trigger Function: Auto-update updated_at column
-- ============================================================================
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Triggers: updated_at auto-update for all relevant tables
-- ============================================================================
CREATE TRIGGER update_tenants_updated_at BEFORE UPDATE ON tenants
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organizations_updated_at BEFORE UPDATE ON organizations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_message_templates_updated_at BEFORE UPDATE ON message_templates
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_messages_updated_at BEFORE UPDATE ON messages
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_user_messages_updated_at BEFORE UPDATE ON user_messages
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_user_message_settings_updated_at BEFORE UPDATE ON user_message_settings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- Schema Summary
-- ============================================================================
-- Total Tables: 12
-- 1. tenants                  - 租户表 (Multi-tenant foundation)
-- 2. organizations            - 组织架构表 (Organization hierarchy)
-- 3. users                    - 用户表 (User accounts)
-- 4. user_organizations       - 用户组织关系表 (User-Org relationships)
-- 5. roles                    - 角色表 (Role definitions)
-- 6. user_roles               - 用户角色关系表 (User-Role relationships)
-- 7. message_templates        - 消息模板表 (Message templates)
-- 8. messages                 - 消息表 (Message definitions)
-- 9. message_target_rules     - 消息接收规则表 (Targeting rules)
-- 10. user_messages           - 用户消息表 (User message instances)
-- 11. user_message_settings   - 用户消息配置表 (User preferences)
-- 12. message_push_logs       - 消息推送记录表 (Push delivery logs)
--
-- Total Indexes: 30+
-- Total Triggers: 7 (auto-update updated_at)
-- ============================================================================
