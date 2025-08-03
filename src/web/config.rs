//! Web 服务器配置
//! 
//! 使用类型安全的环境变量系统进行配置管理

use crate::env::{EnvResult, EnvError, EnvVar};

// MongoDB 配置已移除 - 轻量化版本不再使用数据库

/// Web 服务器配置 - 轻量化版本
#[derive(Debug, Clone)]
pub struct WebConfig {
    /// 绑定地址
    pub bind_addr: String,
    /// 端口
    pub port: u16,
    /// 静态文件目录
    pub static_dir: Option<String>,
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
        
        Ok(Self {
            bind_addr,
            port,
            static_dir,
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
        
        // MongoDB 配置验证已移除
        
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
            }
        })
    }
}
