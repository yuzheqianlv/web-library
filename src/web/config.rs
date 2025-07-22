//! Web 服务器配置

/// MongoDB 配置
#[cfg(feature = "web")]
#[derive(Debug, Clone)]
pub struct MongoConfig {
    /// MongoDB 连接字符串
    pub connection_string: String,
    /// 数据库名称
    pub database_name: String,
    /// 集合名称
    pub collection_name: String,
}

#[cfg(feature = "web")]
impl Default for MongoConfig {
    fn default() -> Self {
        Self {
            connection_string: std::env::var("MONGODB_URL")
                .unwrap_or_else(|_| "mongodb://localhost:27017".to_string()),
            database_name: std::env::var("MONGODB_DATABASE")
                .unwrap_or_else(|_| "monolith".to_string()),
            collection_name: std::env::var("MONGODB_COLLECTION")
                .unwrap_or_else(|_| "html_cache".to_string()),
        }
    }
}

/// Web 服务器配置
#[derive(Debug, Clone)]
pub struct WebConfig {
    /// 绑定地址
    pub bind_addr: String,
    /// 端口
    pub port: u16,
    /// 静态文件目录
    pub static_dir: Option<String>,
    /// MongoDB 配置
    #[cfg(feature = "web")]
    pub mongo_config: Option<MongoConfig>,
    #[cfg(not(feature = "web"))]
    pub mongo_config: Option<()>,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1".to_string(),
            port: 7080,
            static_dir: Some("static".to_string()),
            #[cfg(feature = "web")]
            mongo_config: Some(MongoConfig::default()),
            #[cfg(not(feature = "web"))]
            mongo_config: None,
        }
    }
}
