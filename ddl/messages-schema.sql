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

-- ==========================================
-- Note: tenant/user/organization/role tables
-- are intentionally removed. Identity is
-- provided from JWT token claims.
-- ==========================================

-- 在此处不再使用 tenants/users/organizations/roles 表。
-- 请用 JWT 里的 tenant_id/user_id/org_id 进行身份判断。

-- 旧的 user_roles 索引和注释已移除

-- ============================================================================
-- Table: t_sys_message_templates (消息模板表)
-- ============================================================================
CREATE TABLE t_sys_message_templates (
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

CREATE INDEX idx_t_sys_message_templates_code ON t_sys_message_templates(template_code);
CREATE INDEX idx_t_sys_message_templates_category ON t_sys_message_templates(category);

COMMENT ON TABLE t_sys_message_templates IS '消息模板表';
COMMENT ON COLUMN t_sys_message_templates.category IS '分类:system/business/alarm/interaction';
COMMENT ON COLUMN t_sys_message_templates.priority IS '优先级 1:紧急 2:重要 3:普通 4:低优';
COMMENT ON COLUMN t_sys_message_templates.title_template IS '标题模板 支持变量 {{var}}';
COMMENT ON COLUMN t_sys_message_templates.jump_type IS '跳转类型:url/route/action';
COMMENT ON COLUMN t_sys_message_templates.channels IS '推送渠道 ["web","email","dingtalk"]';

-- ============================================================================
-- Table: t_sys_messages (消息表)
-- ============================================================================
CREATE TABLE t_sys_messages (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    message_code VARCHAR(50) UNIQUE NOT NULL,
    template_id BIGINT REFERENCES t_sys_message_templates(id) ON DELETE SET NULL,
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
    sender_id BIGINT,
    sender_type VARCHAR(20) DEFAULT 'user',
    status SMALLINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_t_sys_messages_tenant ON t_sys_messages(tenant_id);
CREATE INDEX idx_t_sys_messages_template ON t_sys_messages(template_id);
CREATE INDEX idx_t_sys_messages_status ON t_sys_messages(status);
CREATE INDEX idx_t_sys_messages_scheduled ON t_sys_messages(scheduled_at) WHERE scheduled_at IS NOT NULL;
CREATE INDEX idx_t_sys_messages_code ON t_sys_messages(message_code);
CREATE INDEX idx_t_sys_messages_created ON t_sys_messages(created_at DESC);

COMMENT ON TABLE t_sys_messages IS '消息表';
COMMENT ON COLUMN t_sys_messages.send_type IS '发送类型 1:立即 2:定时';
COMMENT ON COLUMN t_sys_messages.sender_type IS 'user/system';
COMMENT ON COLUMN t_sys_messages.status IS '0:待发送 1:已发送 2:已取消 3:失败';

-- ============================================================================
-- Table: t_sys_message_target_rules (消息接收规则表)
-- ============================================================================
CREATE TABLE t_sys_message_target_rules (
    id BIGSERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL REFERENCES t_sys_messages(id) ON DELETE CASCADE,
    target_type VARCHAR(20) NOT NULL,
    target_scope JSONB,
    filter_conditions JSONB,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_t_sys_message_target_rules_message ON t_sys_message_target_rules(message_id);

COMMENT ON TABLE t_sys_message_target_rules IS '消息接收规则表';
COMMENT ON COLUMN t_sys_message_target_rules.target_type IS '目标类型:user/org/role/custom';
COMMENT ON COLUMN t_sys_message_target_rules.target_scope IS '目标范围配置';
COMMENT ON COLUMN t_sys_message_target_rules.filter_conditions IS '筛选条件';

-- ============================================================================
-- Table: t_sys_message_users (消息用户关联表) - Renamed from t_sys_user_messages
-- ============================================================================
CREATE TABLE t_sys_message_users (
    id BIGSERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL REFERENCES t_sys_messages(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL,
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

CREATE INDEX idx_t_sys_message_users_user_read ON t_sys_message_users(user_id, is_read, is_deleted);
CREATE INDEX idx_t_sys_message_users_tenant_user ON t_sys_message_users(tenant_id, user_id);
CREATE INDEX idx_t_sys_message_users_created ON t_sys_message_users(created_at DESC);
CREATE INDEX idx_t_sys_message_users_message ON t_sys_message_users(message_id);

COMMENT ON TABLE t_sys_message_users IS '消息用户关联表';
COMMENT ON COLUMN t_sys_message_users.is_read IS '是否已读';
COMMENT ON COLUMN t_sys_message_users.is_deleted IS '是否删除';
COMMENT ON COLUMN t_sys_message_users.is_pinned IS '是否置顶';

-- ============================================================================
-- Table: t_sys_user_message_settings (用户消息配置表)
-- ============================================================================
CREATE TABLE t_sys_user_message_settings (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
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

COMMENT ON TABLE t_sys_user_message_settings IS '用户消息配置表';
COMMENT ON COLUMN t_sys_user_message_settings.web_enabled IS '站内消息开关';
COMMENT ON COLUMN t_sys_user_message_settings.email_enabled IS '邮件通知开关';
COMMENT ON COLUMN t_sys_user_message_settings.dingtalk_enabled IS '钉钉通知开关';
COMMENT ON COLUMN t_sys_user_message_settings.do_not_disturb IS '免打扰模式';

-- ============================================================================
-- Table: t_sys_message_push_logs (消息推送记录表)
-- ============================================================================
CREATE TABLE t_sys_message_push_logs (
    id BIGSERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    channel VARCHAR(20) NOT NULL,
    status SMALLINT,
    error_msg TEXT,
    pushed_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_t_sys_message_push_logs_message ON t_sys_message_push_logs(message_id);
CREATE INDEX idx_t_sys_message_push_logs_user_channel ON t_sys_message_push_logs(user_id, channel);
CREATE INDEX idx_t_sys_message_push_logs_pushed ON t_sys_message_push_logs(pushed_at DESC);

COMMENT ON TABLE t_sys_message_push_logs IS '消息推送记录表';
COMMENT ON COLUMN t_sys_message_push_logs.channel IS '推送渠道:web/email/dingtalk';
COMMENT ON COLUMN t_sys_message_push_logs.status IS '1:成功 0:失败';

-- ============================================================================
-- Table: t_sys_message_retry_records (消息重试记录表)
-- ============================================================================
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
    status SMALLINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_t_sys_message_retry_records_status_next ON t_sys_message_retry_records(status, next_retry_at) WHERE status = 0;
CREATE INDEX idx_t_sys_message_retry_records_message ON t_sys_message_retry_records(message_id);
CREATE INDEX idx_t_sys_message_retry_records_user ON t_sys_message_retry_records(user_id);

COMMENT ON TABLE t_sys_message_retry_records IS '消息重试记录表';
COMMENT ON COLUMN t_sys_message_retry_records.retry_count IS '当前重试次数';
COMMENT ON COLUMN t_sys_message_retry_records.max_retries IS '最大重试次数';
COMMENT ON COLUMN t_sys_message_retry_records.retry_intervals IS '重试间隔数组(秒),支持指数退避';
COMMENT ON COLUMN t_sys_message_retry_records.next_retry_at IS '下次重试时间';
COMMENT ON COLUMN t_sys_message_retry_records.status IS '0:待重试 1:重试成功 2:进入死信队列';
COMMENT ON COLUMN t_sys_message_retry_records.retry_history IS '重试历史记录 [{"attempt":1,"time":"2024-...","error":"..."}]';

-- ============================================================================
-- Table: t_sys_message_dead_letters (死信队列表)
-- ============================================================================
CREATE TABLE t_sys_message_dead_letters (
    id BIGSERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    channel VARCHAR(20) NOT NULL,
    failed_reason TEXT,
    retry_history JSONB,
    status SMALLINT DEFAULT 0,
    retried_at TIMESTAMPTZ,
    retried_success SMALLINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_t_sys_message_dead_letters_status ON t_sys_message_dead_letters(status);
CREATE INDEX idx_t_sys_message_dead_letters_message ON t_sys_message_dead_letters(message_id);
CREATE INDEX idx_t_sys_message_dead_letters_created ON t_sys_message_dead_letters(created_at DESC);

COMMENT ON TABLE t_sys_message_dead_letters IS '死信队列表';
COMMENT ON COLUMN t_sys_message_dead_letters.status IS '0:待处理 1:已重试 2:已放弃';
COMMENT ON COLUMN t_sys_message_dead_letters.failed_reason IS '最终失败原因';
COMMENT ON COLUMN t_sys_message_dead_letters.retried_at IS '管理员重试时间';
COMMENT ON COLUMN t_sys_message_dead_letters.retried_success IS '管理员重试是否成功';

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
CREATE TRIGGER update_t_sys_message_templates_updated_at BEFORE UPDATE ON t_sys_message_templates
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_t_sys_messages_updated_at BEFORE UPDATE ON t_sys_messages
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_t_sys_message_users_updated_at BEFORE UPDATE ON t_sys_message_users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_t_sys_user_message_settings_updated_at BEFORE UPDATE ON t_sys_user_message_settings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_t_sys_message_retry_records_updated_at BEFORE UPDATE ON t_sys_message_retry_records
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_t_sys_message_dead_letters_updated_at BEFORE UPDATE ON t_sys_message_dead_letters
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- Schema Summary
-- ============================================================================
-- Total Tables: 8
-- 1. t_sys_message_templates       - 消息模板表
-- 2. t_sys_messages                - 消息表
-- 3. t_sys_message_target_rules    - 消息接收规则表
-- 4. t_sys_message_users           - 消息用户关联表 (原名: t_sys_user_messages)
-- 5. t_sys_user_message_settings   - 用户消息配置表
-- 6. t_sys_message_push_logs       - 消息推送记录表
-- 7. t_sys_message_retry_records   - 消息重试记录表 (新增)
-- 8. t_sys_message_dead_letters    - 死信队列表 (新增)
--
-- Total Indexes: 20+
-- Total Triggers: 6 (auto-update updated_at)
-- ============================================================================
