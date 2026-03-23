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
    pub channel_config: ChannelConfig,
}

/// 渠道开关配置 - 从环境变量读取
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelConfig {
    pub websocket_enabled: bool,
    pub email_enabled: bool,
    pub dingtalk_enabled: bool,
    pub sms_enabled: bool,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            websocket_enabled: true,
            email_enabled: false,
            dingtalk_enabled: false,
            sms_enabled: false,
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Config {
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()?,
            database_url: env::var("DATABASE_URL")?.trim().to_string(),
            redis_url: env::var("REDIS_URL")?.trim().to_string(),
            rabbitmq_url: env::var("RABBITMQ_URL").unwrap_or_else(|_| "".to_string()),
            jwt_secret: env::var("JWT_SECRET")?.trim().to_string(),
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
            channel_config: ChannelConfig::from_env(),
        })
    }
}

impl ChannelConfig {
    /// 从环境变量读取渠道开关配置
    pub fn from_env() -> Self {
        Self {
            websocket_enabled: env::var("CHANNEL_WEBSOCKET_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            email_enabled: env::var("CHANNEL_EMAIL_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            dingtalk_enabled: env::var("CHANNEL_DINGTALK_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            sms_enabled: env::var("CHANNEL_SMS_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
        }
    }
}
