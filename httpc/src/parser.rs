use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Request {
    pub fn resolve_body(&mut self, base_dir: &Path) -> Result<(), String> {
        if let Some(path_str) = extract_file_ref_path(&self.body) {
            let resolved: PathBuf = base_dir.join(path_str).components().collect();
            self.body = std::fs::read_to_string(&resolved)
                .map_err(|e| format!("failed to read body file {}: {e}", resolved.display()))?;
        }
        Ok(())
    }
}

fn extract_file_ref_path(body: &str) -> Option<&str> {
    body.strip_prefix("< ").and_then(|rest| {
        let first_line = rest.lines().next()?.trim();
        (!first_line.is_empty()).then_some(first_line)
    })
}

const HTTP_METHODS: &[&str] = &[
    "OPTIONS",
    "GET",
    "HEAD",
    "POST",
    "PUT",
    "DELETE",
    "TRACE",
    "CONNECT",
    "PATCH",
    "LIST",
    "GRAPHQL",
    "WEBSOCKET",
];

pub fn parse_request_at(content: &str, line: usize) -> Result<Request, String> {
    let lines: Vec<&str> = content.lines().collect();

    if line < 1 || line > lines.len() {
        return Err(format!(
            "line {} is out of range (file has {} lines)",
            line,
            lines.len()
        ));
    }

    let target_idx = line - 1;

    let mut block_start = 0;
    for i in (0..=target_idx).rev() {
        if lines[i].starts_with("###") {
            block_start = i + 1;
            break;
        }
    }

    let mut block_end = lines.len();
    for (i, line) in lines.iter().enumerate().skip(target_idx + 1) {
        if line.starts_with("###") {
            block_end = i;
            break;
        }
    }

    let block = &lines[block_start..block_end];

    let mut i = 0;
    while i < block.len() {
        let trimmed = block[i].trim();
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with("//")
            || trimmed.starts_with('@')
        {
            i += 1;
        } else {
            break;
        }
    }

    if i >= block.len() {
        return Err("no method line found in request block".to_string());
    }

    let method_line = block[i];
    let mut parts = method_line.split_whitespace();
    let method = parts.next().ok_or("missing method")?.to_string();
    let url = parts.next().ok_or("missing URL")?.to_string();
    i += 1;

    let mut headers = Vec::new();
    while i < block.len() {
        let header_line = block[i];
        if header_line.trim().is_empty() {
            break;
        }
        if let Some(colon_pos) = header_line.find(':') {
            let name = header_line[..colon_pos].trim().to_string();
            let value = header_line[colon_pos + 1..].trim().to_string();
            headers.push((name, value));
        }
        i += 1;
    }

    if i < block.len() {
        i += 1;
    }

    let body = block[i..].join("\n").trim().to_string();

    let vars = collect_variable_definitions(content);
    let url = substitute_variables(&url, &vars);
    let headers = headers
        .into_iter()
        .map(|(name, value)| (name, substitute_variables(&value, &vars)))
        .collect();
    let body = substitute_variables(&body, &vars);

    Ok(Request {
        method,
        url,
        headers,
        body,
    })
}

fn collect_variable_definitions(content: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    for line in content.lines() {
        if line.starts_with("###") || is_method_line(line) {
            break;
        }
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix('@') else {
            continue;
        };
        let Some(eq_pos) = rest.find('=') else {
            continue;
        };
        let name = rest[..eq_pos].trim();
        let value = rest[eq_pos + 1..].trim();
        if is_valid_var_name(name) {
            vars.insert(name.to_string(), value.to_string());
        }
    }
    vars
}

fn is_method_line(line: &str) -> bool {
    let first_word = line.split_whitespace().next();
    matches!(first_word, Some(w) if HTTP_METHODS.contains(&w))
}

fn is_valid_var_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '_' | '.' | '-')
                || matches!(c, '\u{00A1}'..='\u{FFFF}')
        })
}

fn substitute_variables(text: &str, vars: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(open) = remaining.find("{{") {
        result.push_str(&remaining[..open]);
        let after_open = &remaining[open + 2..];
        if let Some(close) = after_open.find("}}") {
            let raw_name = &after_open[..close];
            let name = raw_name.trim();
            if let Some(value) = vars.get(name) {
                result.push_str(value);
            } else {
                result.push_str("{{");
                result.push_str(raw_name);
                result.push_str("}}");
            }
            remaining = &after_open[close + 2..];
        } else {
            result.push_str(&remaining[open..]);
            return result;
        }
    }
    result.push_str(remaining);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_get_request() {
        let content = "GET https://example.com/api\nAccept: application/json\n";
        let req = parse_request_at(content, 1).unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "https://example.com/api");
        assert_eq!(
            req.headers,
            vec![("Accept".to_string(), "application/json".to_string())]
        );
        assert_eq!(req.body, "");
    }

    #[test]
    fn parses_post_with_json_body() {
        let content = "POST https://example.com/users\nContent-Type: application/json\n\n{\"name\":\"alice\"}\n";
        let req = parse_request_at(content, 1).unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.url, "https://example.com/users");
        assert_eq!(
            req.headers,
            vec![("Content-Type".to_string(), "application/json".to_string())]
        );
        assert_eq!(req.body, "{\"name\":\"alice\"}");
    }

    #[test]
    fn parses_url_with_query_string() {
        let content = "GET /api?role=admin&limit=10\n";
        let req = parse_request_at(content, 1).unwrap();
        assert_eq!(req.url, "/api?role=admin&limit=10");
    }

    #[test]
    fn ignores_http_version_after_url() {
        let content = "GET /api HTTP/1.1\n";
        let req = parse_request_at(content, 1).unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "/api");
    }

    #[test]
    fn handles_no_body() {
        let content = "DELETE /users/42\n";
        let req = parse_request_at(content, 1).unwrap();
        assert_eq!(req.method, "DELETE");
        assert_eq!(req.url, "/users/42");
        assert!(req.headers.is_empty());
        assert_eq!(req.body, "");
    }

    #[test]
    fn handles_multiline_body() {
        let content =
            "POST /a\nContent-Type: application/xml\n\n<root>\n  <child>value</child>\n</root>\n";
        let req = parse_request_at(content, 1).unwrap();
        assert_eq!(req.body, "<root>\n  <child>value</child>\n</root>");
    }

    #[test]
    fn parses_multiple_headers() {
        let content =
            "GET /api\nAccept: application/json\nAuthorization: Bearer token\nX-Trace-Id: abc\n";
        let req = parse_request_at(content, 1).unwrap();
        assert_eq!(
            req.headers,
            vec![
                ("Accept".to_string(), "application/json".to_string()),
                ("Authorization".to_string(), "Bearer token".to_string()),
                ("X-Trace-Id".to_string(), "abc".to_string()),
            ]
        );
    }

    #[test]
    fn header_value_can_contain_colon() {
        let content = "GET /api\nContent-Type: application/json; charset=utf-8\n";
        let req = parse_request_at(content, 1).unwrap();
        assert_eq!(req.headers[0].1, "application/json; charset=utf-8");
    }

    #[test]
    fn skips_comments_above_method_line() {
        let content = "# this is a comment\n// also a comment\nGET /api\n";
        let req = parse_request_at(content, 1).unwrap();
        assert_eq!(req.method, "GET");
    }

    #[test]
    fn parses_request_within_separated_blocks() {
        let content = "### first\nGET /a\n\n### second\nPOST /b\n";
        let req = parse_request_at(content, 5).unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.url, "/b");
    }

    #[test]
    fn resolves_request_when_clicked_on_body_line() {
        let content =
            "POST /users\nContent-Type: application/json\n\n{\n  \"name\": \"alice\"\n}\n";
        let req = parse_request_at(content, 5).unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.body, "{\n  \"name\": \"alice\"\n}");
    }

    #[test]
    fn resolves_request_when_clicked_on_header_line() {
        let content = "GET /api\nAccept: application/json\nX-Custom: value\n";
        let req = parse_request_at(content, 2).unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.headers.len(), 2);
    }

    #[test]
    fn errors_on_out_of_range_line() {
        let content = "GET /api\n";
        let result = parse_request_at(content, 5);
        assert!(result.is_err());
    }

    #[test]
    fn errors_on_block_without_method_line() {
        let content = "### only separator\n# comment line\n\n";
        let result = parse_request_at(content, 2);
        assert!(result.is_err());
    }

    #[test]
    fn expands_variable_in_url() {
        let content = "@host = https://api.example.com\n\nGET {{host}}/users\n";
        let req = parse_request_at(content, 3).unwrap();
        assert_eq!(req.url, "https://api.example.com/users");
    }

    #[test]
    fn expands_variable_in_header_value() {
        let content = "@token = abc123\n\nGET /api\nAuthorization: Bearer {{token}}\n";
        let req = parse_request_at(content, 3).unwrap();
        assert_eq!(req.headers[0].1, "Bearer abc123");
    }

    #[test]
    fn expands_variable_in_body() {
        let content = "@name = alice\n\nPOST /users\nContent-Type: application/json\n\n{\"name\":\"{{name}}\"}\n";
        let req = parse_request_at(content, 3).unwrap();
        assert_eq!(req.body, "{\"name\":\"alice\"}");
    }

    #[test]
    fn unknown_variable_left_as_is() {
        let content = "GET /api/{{notDefined}}\n";
        let req = parse_request_at(content, 1).unwrap();
        assert_eq!(req.url, "/api/{{notDefined}}");
    }

    #[test]
    fn multiple_variables_in_one_string() {
        let content =
            "@host = api.example.com\n@port = 8080\n\nGET https://{{host}}:{{port}}/users\n";
        let req = parse_request_at(content, 4).unwrap();
        assert_eq!(req.url, "https://api.example.com:8080/users");
    }

    #[test]
    fn variable_below_first_method_line_is_not_collected() {
        // Lines after the first method line are part of headers/body and must not
        // be picked up as variable definitions.
        let content = "GET {{host}}/api\n\n@host = https://api.example.com\n";
        let req = parse_request_at(content, 1).unwrap();
        assert_eq!(req.url, "{{host}}/api");
    }

    #[test]
    fn variable_after_first_separator_is_not_collected() {
        // tree-sitter and IntelliJ both treat @var lines after the first ### as body
        // content. The skip-loop must still avoid them when looking for the method line,
        // but they must not be collected as variable definitions.
        let content = "### foo\n@host = api.example.com\nGET {{host}}/api\n";
        let req = parse_request_at(content, 3).unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "{{host}}/api");
    }

    #[test]
    fn variable_definition_with_or_without_whitespace() {
        let content = "@name=value\n@spaced  =  spaced-value\n\nGET /a/{{name}}/{{spaced}}\n";
        let req = parse_request_at(content, 3).unwrap();
        assert_eq!(req.url, "/a/value/spaced-value");
    }

    #[test]
    fn variable_name_with_dot_and_hyphen() {
        let content = "@my.var = one\n@host-name = two\n\nGET /{{my.var}}/{{host-name}}\n";
        let req = parse_request_at(content, 4).unwrap();
        assert_eq!(req.url, "/one/two");
    }

    #[test]
    fn variable_name_with_unicode() {
        let content = "@ホスト = api.example.com\n\nGET https://{{ホスト}}/api\n";
        let req = parse_request_at(content, 3).unwrap();
        assert_eq!(req.url, "https://api.example.com/api");
    }

    #[test]
    fn variable_name_with_dollar_is_rejected() {
        let content = "@$custom = ignored\n\nGET /{{$custom}}\n";
        let req = parse_request_at(content, 3).unwrap();
        assert_eq!(req.url, "/{{$custom}}");
    }

    #[test]
    fn extract_file_ref_path_recognizes_relative_path() {
        assert_eq!(extract_file_ref_path("< ./file.json"), Some("./file.json"));
    }

    #[test]
    fn extract_file_ref_path_recognizes_absolute_path() {
        assert_eq!(
            extract_file_ref_path("< /tmp/data.json"),
            Some("/tmp/data.json")
        );
    }

    #[test]
    fn extract_file_ref_path_strips_trailing_whitespace() {
        assert_eq!(
            extract_file_ref_path("< ./file.json  "),
            Some("./file.json")
        );
    }

    #[test]
    fn extract_file_ref_path_takes_first_line_only() {
        assert_eq!(
            extract_file_ref_path("< ./file.json\nignored content"),
            Some("./file.json")
        );
    }

    #[test]
    fn extract_file_ref_path_does_not_match_xml_open_tag() {
        assert_eq!(extract_file_ref_path("<user>foo</user>"), None);
    }

    #[test]
    fn extract_file_ref_path_does_not_match_xml_declaration() {
        assert_eq!(extract_file_ref_path("<?xml version=\"1.0\"?>"), None);
    }

    #[test]
    fn extract_file_ref_path_does_not_match_close_tag() {
        assert_eq!(extract_file_ref_path("</close>"), None);
    }

    #[test]
    fn extract_file_ref_path_returns_none_on_empty_path() {
        assert_eq!(extract_file_ref_path("< "), None);
        assert_eq!(extract_file_ref_path("<"), None);
    }

    #[test]
    fn extract_file_ref_path_returns_none_on_empty_body() {
        assert_eq!(extract_file_ref_path(""), None);
    }
}
