use std::time::{Duration, Instant};

use ureq::{Agent, http};

use crate::parser::Request;

const TIMEOUT_CONNECT: Duration = Duration::from_secs(10);
const TIMEOUT_RECV_RESPONSE: Duration = Duration::from_secs(600);

pub struct Response {
    pub http_version: String,
    pub status_code: u16,
    pub reason_phrase: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub elapsed: Duration,
}

pub fn send(req: &Request) -> Result<Response, String> {
    let agent: Agent = Agent::config_builder()
        .http_status_as_error(false)
        .timeout_connect(Some(TIMEOUT_CONNECT))
        .timeout_recv_response(Some(TIMEOUT_RECV_RESPONSE))
        .build()
        .into();

    let method = http::Method::from_bytes(req.method.as_bytes())
        .map_err(|e| format!("invalid HTTP method {:?}: {e}", req.method))?;

    let mut builder = http::Request::builder().method(method).uri(&req.url);
    for (name, value) in &req.headers {
        builder = builder.header(name, value);
    }
    let request = builder
        .body(req.body.clone())
        .map_err(|e| format!("failed to build request: {e}"))?;

    let start = Instant::now();
    let mut response = agent
        .run(request)
        .map_err(|e| format!("request failed: {e}"))?;
    let elapsed = start.elapsed();

    let http_version = format_version(response.version());
    let status = response.status();
    let status_code = status.as_u16();
    let reason_phrase = status.canonical_reason().unwrap_or("").to_string();

    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                value.to_str().unwrap_or("").to_string(),
            )
        })
        .collect();

    let body = response
        .body_mut()
        .read_to_vec()
        .map_err(|e| format!("failed to read response body: {e}"))?;

    Ok(Response {
        http_version,
        status_code,
        reason_phrase,
        headers,
        body,
        elapsed,
    })
}

fn format_version(v: http::Version) -> String {
    match v {
        http::Version::HTTP_09 => "HTTP/0.9",
        http::Version::HTTP_10 => "HTTP/1.0",
        http::Version::HTTP_11 => "HTTP/1.1",
        http::Version::HTTP_2 => "HTTP/2",
        http::Version::HTTP_3 => "HTTP/3",
        _ => "HTTP/?",
    }
    .to_string()
}
