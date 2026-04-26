pub struct Response {
    pub status_line: String,
    pub headers: Vec<String>,
    pub body: String,
}

pub fn split_response(raw: &str) -> Response {
    let (header_section, body) = if let Some(p) = raw.find("\r\n\r\n") {
        (&raw[..p], &raw[p + 4..])
    } else if let Some(p) = raw.find("\n\n") {
        (&raw[..p], &raw[p + 2..])
    } else {
        (raw, "")
    };

    let mut lines = header_section.lines();
    let status_line = lines.next().unwrap_or("").to_string();
    let headers = lines.map(String::from).collect();

    Response {
        status_line,
        headers,
        body: body.trim_end().to_string(),
    }
}

pub fn find_content_type(headers: &[String]) -> Option<String> {
    headers.iter().find_map(|h| {
        let (name, value) = h.split_once(':')?;
        if name.eq_ignore_ascii_case("content-type") {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}

pub fn pretty(body: &str, content_type: Option<&str>) -> String {
    match content_type {
        Some(ct) if is_json(ct) => pretty_json(body),
        Some(ct) if is_xml(ct) => pretty_xml(body),
        _ => body.to_string(),
    }
}

pub fn parse_status_code(status_line: &str) -> &str {
    status_line.split_whitespace().nth(1).unwrap_or("?")
}

fn is_json(ct: &str) -> bool {
    ct.starts_with("application/json")
        || (ct.starts_with("application/") && ct.contains("+json"))
}

fn is_xml(ct: &str) -> bool {
    ct.starts_with("application/xml")
        || ct.starts_with("text/xml")
        || (ct.starts_with("application/") && ct.contains("+xml"))
}

fn pretty_json(input: &str) -> String {
    serde_json::from_str::<serde_json::Value>(input)
        .ok()
        .and_then(|v| serde_json::to_string_pretty(&v).ok())
        .unwrap_or_else(|| input.to_string())
}

fn pretty_xml(input: &str) -> String {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;
    use quick_xml::writer::Writer;

    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);

    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(event) => {
                if writer.write_event(event).is_err() {
                    return input.to_string();
                }
            }
            Err(_) => return input.to_string(),
        }
        buf.clear();
    }

    String::from_utf8(writer.into_inner()).unwrap_or_else(|_| input.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_response_separates_status_headers_body() {
        let raw = "HTTP/2 200\r\nContent-Type: application/json\r\n\r\n{\"a\":1}\n";
        let resp = split_response(raw);
        assert_eq!(resp.status_line, "HTTP/2 200");
        assert_eq!(
            resp.headers,
            vec!["Content-Type: application/json".to_string()]
        );
        assert_eq!(resp.body, "{\"a\":1}");
    }

    #[test]
    fn split_response_handles_unix_line_endings() {
        let raw = "HTTP/1.1 200 OK\nContent-Length: 7\n\n{\"a\":1}";
        let resp = split_response(raw);
        assert_eq!(resp.status_line, "HTTP/1.1 200 OK");
        assert_eq!(resp.body, "{\"a\":1}");
    }

    #[test]
    fn split_response_with_no_body_section() {
        let raw = "HTTP/2 204\r\n\r\n";
        let resp = split_response(raw);
        assert_eq!(resp.status_line, "HTTP/2 204");
        assert_eq!(resp.body, "");
    }

    #[test]
    fn find_content_type_finds_header() {
        let headers = vec![
            "Date: Sun, 26 Apr 2026 09:00:00 GMT".to_string(),
            "Content-Type: application/json".to_string(),
        ];
        assert_eq!(
            find_content_type(&headers),
            Some("application/json".to_string())
        );
    }

    #[test]
    fn find_content_type_is_case_insensitive() {
        let headers = vec!["content-type: application/json".to_string()];
        assert_eq!(
            find_content_type(&headers),
            Some("application/json".to_string())
        );
    }

    #[test]
    fn find_content_type_returns_none_when_absent() {
        let headers = vec!["Date: ...".to_string()];
        assert_eq!(find_content_type(&headers), None);
    }

    #[test]
    fn is_json_recognizes_application_json() {
        assert!(is_json("application/json"));
        assert!(is_json("application/json; charset=utf-8"));
    }

    #[test]
    fn is_json_recognizes_plus_json_suffix() {
        assert!(is_json("application/vnd.api+json"));
        assert!(is_json("application/hal+json"));
    }

    #[test]
    fn is_xml_recognizes_application_xml_and_text_xml() {
        assert!(is_xml("application/xml"));
        assert!(is_xml("text/xml"));
        assert!(is_xml("application/atom+xml"));
    }

    #[test]
    fn pretty_json_indents_compact_input() {
        let input = r#"{"a":1,"b":[1,2,3]}"#;
        let output = pretty_json(input);
        assert!(output.contains('\n'));
        assert!(output.contains("  "));
    }

    #[test]
    fn pretty_json_returns_input_on_invalid() {
        let input = "not json";
        assert_eq!(pretty_json(input), "not json");
    }

    #[test]
    fn pretty_xml_indents_compact_input() {
        let input = "<root><a>1</a><b>2</b></root>";
        let output = pretty_xml(input);
        assert!(output.contains('\n'));
        assert!(output.contains("  "));
    }

    #[test]
    fn parse_status_code_extracts_numeric_code() {
        assert_eq!(parse_status_code("HTTP/2 200"), "200");
        assert_eq!(parse_status_code("HTTP/1.1 404 Not Found"), "404");
    }
}
