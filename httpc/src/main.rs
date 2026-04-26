use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

mod parser;
mod response;

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

    let start = std::time::Instant::now();
    let output = cmd.output().expect("failed to spawn curl");
    let elapsed = start.elapsed();

    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let resp = response::split_response(&raw);
    let content_type = response::find_content_type(&resp.headers);

    println!("{}", resp.status_line);
    for h in &resp.headers {
        println!("{h}");
    }
    println!();

    let pretty = response::pretty(&resp.body, content_type.as_deref());
    println!("{pretty}");
    println!();

    println!(
        "Response code: {}; Time: {:.3}s; Content length: {} bytes",
        response::parse_status_code(&resp.status_line),
        elapsed.as_secs_f64(),
        resp.body.len(),
    );

    output.status.code().unwrap_or(1)
}
