use clap::Parser;
use std::fs;
use std::path::{Path, PathBuf};

mod client;
mod formatter;
mod parser;

/// Zed HTTP Client — backend CLI that executes requests defined in .http files.
#[derive(Parser)]
#[command(name = "httpc", version, about)]
struct Args {
    /// Path to the .http file
    #[arg(long)]
    file: PathBuf,

    /// 1-based line number; the request containing this line is executed
    #[arg(long)]
    line: usize,
}

fn main() {
    if let Err(msg) = run() {
        eprintln!("httpc: {msg}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse();

    let content = fs::read_to_string(&args.file)
        .map_err(|e| format!("failed to read {}: {e}", args.file.display()))?;
    let mut req = parser::parse_request_at(&content, args.line)
        .map_err(|e| format!("parse error at line {}: {e}", args.line))?;

    let base_dir = args.file.parent().unwrap_or(Path::new("."));
    req.resolve_body(base_dir)?;

    formatter::print_request(&req);
    let resp = client::send(&req)?;
    formatter::print_response(&resp);

    Ok(())
}
