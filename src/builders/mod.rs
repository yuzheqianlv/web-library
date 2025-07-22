//! # 构建器模块
//!
//! 这个模块包含用于构建各种输出格式的构建器：
//!
//! - HTML模板构建器
//! - 响应格式构建器
//!
//! # 模块组织
//!
//! - `html_builder` - HTML模板构建和渲染（需要web特性）

#[cfg(feature = "web")]
pub mod html_builder;

// Re-export commonly used items for convenience
#[cfg(feature = "web")]
pub use html_builder::*;
