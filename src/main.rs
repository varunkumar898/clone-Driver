use diskclone::logging::{init_logging, LogConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log_config = LogConfig::default();
    init_logging(&log_config)?;

    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    println!("=== DiskClone Linux Storage Engine ===");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));

    match command {
        "scan" => {
            println!("Scanning for available block devices...");
            // Skeleton scaffold
            println!("Device discovery scaffold ready.");
        }
        "version" | "-v" | "--version" => {
            println!("diskclone v{}", env!("CARGO_PKG_VERSION"));
        }
        _ => {
            println!("Usage: diskclone [command]");
            println!();
            println!("Commands:");
            println!("  scan       Scan and list eligible block devices");
            println!("  version    Print version information");
            println!("  help       Print this help message");
        }
    }

    Ok(())
}
