//! 环境变量文档生成工具
//! 
//! 此工具用于生成环境变量的Markdown文档

use monolith::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", env::generate_env_docs());
    Ok(())
}