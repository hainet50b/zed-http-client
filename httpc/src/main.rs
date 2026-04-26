use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

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
    let args = Args::parse();
    let content = fs::read_to_string(&args.file)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", args.file.display(), e));
    let req = parser::parse_request_at(&content, args.line)
        .unwrap_or_else(|e| panic!("parse error: {}", e));

    print_request(&req);
    let exit_code = invoke_curl(&req);
    std::process::exit(exit_code);
}

fn print_request(req: &parser::Request) {
    println!("{} {}", req.method, req.url);
    for (name, value) in &req.headers {
        println!("{name}: {value}");
    }
    if !req.body.is_empty() {
        println!();
        println!("{}", req.body);
    }
    println!();
}

fn invoke_curl(req: &parser::Request) -> i32 {
    let mut cmd = Command::new("curl");
    cmd.arg("-sS")
        .arg("-i")
        .arg("-X")
        .arg(&req.method)
        .arg(&req.url);
    for (name, value) in &req.headers {
        cmd.arg("-H").arg(format!("{name}: {value}"));
    }
    if !req.body.is_empty() {
        cmd.arg("--data-raw").arg(&req.body);
    }
    cmd.arg("-w").arg(
        "\nResponse code: %{http_code}; Time: %{time_total}s; Content length: %{size_download} bytes\n",
    );

    cmd.status()
        .expect("failed to spawn curl")
        .code()
        .unwrap_or(1)
}
