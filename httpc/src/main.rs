use clap::Parser;
use std::path::PathBuf;

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

    let req = parser::parse_request_at(&args.file, args.line)?;

    formatter::print_request(&req);
    let resp = client::send(&req)?;
    formatter::print_response(&resp);

    Ok(())
}
