//! Web 服务器主程序入口

#[cfg(feature = "web")]
use monolith::core::MonolithOptions;
#[cfg(feature = "web")]
use monolith::web::{WebConfig, WebServer};

#[cfg(feature = "web")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 解析命令行参数
    let args: Vec<String> = std::env::args().collect();

    let mut bind_addr = "127.0.0.1".to_string();
    let mut port = 7080u16;

    // 简单的命令行参数解析
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--bind" | "-b" => {
                if i + 1 < args.len() {
                    bind_addr = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --bind requires an address");
                    std::process::exit(1);
                }
            }
            "--port" | "-p" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().unwrap_or_else(|_| {
                        eprintln!("Error: Invalid port number");
                        std::process::exit(1);
                    });
                    i += 2;
                } else {
                    eprintln!("Error: --port requires a port number");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => {
                eprintln!("Error: Unknown argument: {}", args[i]);
                print_help();
                std::process::exit(1);
            }
        }
    }

    // 创建 Monolith 选项
    let mut monolith_options = MonolithOptions::default();
    monolith_options.silent = true; // Web 模式下静默运行
    monolith_options.output_format = monolith::core::MonolithOutputFormat::HTML;

    // 创建 Web 配置
    let web_config = WebConfig {
        bind_addr,
        port,
        static_dir: Some("static".to_string()),
        redis_config: Some(monolith::redis_cache::RedisCacheConfig::default()),
    };

    // 启动 Web 服务器
    let server = WebServer::new(web_config, monolith_options);
    server.start().await?;

    Ok(())
}

#[cfg(feature = "web")]
fn print_help() {
    println!("Monolith Web Server");
    println!();
    println!("USAGE:");
    println!("    monolith-web [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -b, --bind <ADDRESS>     Bind address [default: 127.0.0.1]");
    println!("    -p, --port <PORT>        Port number [default: 7080]");
    println!("    -h, --help               Print help information");
    println!();
    println!("EXAMPLES:");
    println!("    monolith-web");
    println!("    monolith-web --bind 0.0.0.0 --port 3000");
}

#[cfg(not(feature = "web"))]
fn main() {
    eprintln!("Error: Web feature not enabled. Please compile with --features web");
    std::process::exit(1);
}
