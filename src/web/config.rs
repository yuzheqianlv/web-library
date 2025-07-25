//! Web 服务器配置
//! 
//! 使用类型安全的环境变量系统进行配置管理

use crate::env::{EnvResult, EnvError, EnvVar};

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
impl MongoConfig {
    /// 从环境变量创建配置
    pub fn from_env() -> EnvResult<Self> {
        use crate::env::mongodb;
        
        Ok(Self {
            connection_string: mongodb::ConnectionString::get()?,
            database_name: mongodb::DatabaseName::get()?,
            collection_name: mongodb::CollectionName::get()?,
        })
    }
    
    /// 验证配置
    pub fn validate(&self) -> EnvResult<()> {
        if self.connection_string.is_empty() {
            return Err(EnvError {
                variable: "MONGODB_URL".to_string(),
                message: "Connection string cannot be empty".to_string(),
            });
        }
        
        if self.database_name.is_empty() {
            return Err(EnvError {
                variable: "MONGODB_DATABASE".to_string(),
                message: "Database name cannot be empty".to_string(),
            });
        }
        
        if self.collection_name.is_empty() {
            return Err(EnvError {
                variable: "MONGODB_COLLECTION".to_string(),
                message: "Collection name cannot be empty".to_string(),
            });
        }
        
        Ok(())
    }
}

#[cfg(feature = "web")]
impl Default for MongoConfig {
    fn default() -> Self {
        Self::from_env().unwrap_or_else(|e| {
            tracing::warn!("Failed to load MongoDB config from environment: {}. Using defaults.", e);
            Self {
                connection_string: "mongodb://localhost:27017".to_string(),
                database_name: "monolith".to_string(),
                collection_name: "html_cache".to_string(),
            }
        })
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

impl WebConfig {
    /// 从环境变量创建配置
    pub fn from_env() -> EnvResult<Self> {
        use crate::env::web;
        
        let bind_addr = web::BindAddress::get()?;
        let port = web::Port::get()?;
        let static_dir_str = web::StaticDir::get()?;
        let static_dir = if static_dir_str.is_empty() { 
            None 
        } else { 
            Some(static_dir_str) 
        };
        
        #[cfg(feature = "web")]
        let mongo_config = Some(MongoConfig::from_env()?);
        #[cfg(not(feature = "web"))]
        let mongo_config = None;
        
        Ok(Self {
            bind_addr,
            port,
            static_dir,
            mongo_config,
        })
    }
    
    /// 验证配置
    pub fn validate(&self) -> EnvResult<()> {
        // 验证绑定地址
        if self.bind_addr.is_empty() {
            return Err(EnvError {
                variable: "MONOLITH_WEB_BIND_ADDRESS".to_string(),
                message: "Bind address cannot be empty".to_string(),
            });
        }
        
        // 验证端口范围
        if self.port == 0 {
            return Err(EnvError {
                variable: "MONOLITH_WEB_PORT".to_string(),
                message: "Port cannot be 0".to_string(),
            });
        }
        
        // 验证静态文件目录（如果设置）
        if let Some(ref static_dir) = self.static_dir {
            let path = std::path::Path::new(static_dir);
            if !path.exists() {
                tracing::warn!("Static directory '{}' does not exist", static_dir);
            }
        }
        
        // 验证MongoDB配置
        #[cfg(feature = "web")]
        if let Some(ref mongo_config) = self.mongo_config {
            mongo_config.validate()?;
        }
        
        Ok(())
    }
    
    /// 获取完整的监听地址
    pub fn listen_address(&self) -> String {
        format!("{}:{}", self.bind_addr, self.port)
    }
    
    /// 检查是否为本地开发模式
    pub fn is_development(&self) -> bool {
        use crate::env::core;
        core::Mode::get()
            .map(|mode| mode == "development")
            .unwrap_or(false)
    }
}

impl Default for WebConfig {
    fn default() -> Self {
        Self::from_env().unwrap_or_else(|e| {
            tracing::warn!("Failed to load web config from environment: {}. Using defaults.", e);
            Self {
                bind_addr: "127.0.0.1".to_string(),
                port: 7080,
                static_dir: Some("static".to_string()),
                #[cfg(feature = "web")]
                mongo_config: Some(MongoConfig::default()),
                #[cfg(not(feature = "web"))]
                mongo_config: None,
            }
        })
    }
}
