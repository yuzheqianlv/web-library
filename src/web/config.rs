//! Web 服务器配置

#[cfg(feature = "web")]
use crate::redis_cache::RedisCacheConfig;

/// Web 服务器配置
#[derive(Debug, Clone)]
pub struct WebConfig {
    /// 绑定地址
    pub bind_addr: String,
    /// 端口
    pub port: u16,
    /// 静态文件目录
    pub static_dir: Option<String>,
    /// Redis 缓存配置
    #[cfg(feature = "web")]
    pub redis_config: Option<RedisCacheConfig>,
    #[cfg(not(feature = "web"))]
    pub redis_config: Option<()>,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1".to_string(),
            port: 7080,
            static_dir: Some("static".to_string()),
            #[cfg(feature = "web")]
            redis_config: Some(RedisCacheConfig::default()),
            #[cfg(not(feature = "web"))]
            redis_config: None,
        }
    }
}