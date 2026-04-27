use crate::client::Response;
use crate::parser::Request;

pub fn print_request(req: &Request) {
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

pub fn print_response(resp: &Response) {
    if resp.reason_phrase.is_empty() {
        println!("{} {}", resp.http_version, resp.status_code);
    } else {
        println!(
            "{} {} {}",
            resp.http_version, resp.status_code, resp.reason_phrase
        );
    }
    for (name, value) in &resp.headers {
        println!("{name}: {value}");
    }
    println!();

    let body_str = String::from_utf8_lossy(&resp.body);
    let content_type = find_content_type(&resp.headers);
    let pretty = pretty_body(&body_str, content_type.as_deref());
    println!("{pretty}");
    println!();

    println!(
        "Response code: {}; Time: {}ms; Content length: {} bytes",
        if resp.reason_phrase.is_empty() {
            resp.status_code.to_string()
        } else {
            format!("{} ({})", resp.status_code, resp.reason_phrase)
        },
        resp.elapsed.as_millis(),
        resp.body.len(),
    );
}

fn find_content_type(headers: &[(String, String)]) -> Option<String> {
    headers.iter().find_map(|(name, value)| {
        if name.eq_ignore_ascii_case("content-type") {
            Some(value.clone())
        } else {
            None
        }
    })
}

fn pretty_body(body: &str, content_type: Option<&str>) -> String {
    match content_type {
        Some(ct) if is_json(ct) => pretty_json(body),
        Some(ct) if is_xml(ct) => pretty_xml(body),
        _ => body.to_string(),
    }
}

fn is_json(ct: &str) -> bool {
    ct.starts_with("application/json") || (ct.starts_with("application/") && ct.contains("+json"))
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
    fn find_content_type_finds_header() {
        let headers = vec![
            (
                "Date".to_string(),
                "Sun, 26 Apr 2026 09:00:00 GMT".to_string(),
            ),
            ("Content-Type".to_string(), "application/json".to_string()),
        ];
        assert_eq!(
            find_content_type(&headers),
            Some("application/json".to_string())
        );
    }

    #[test]
    fn find_content_type_is_case_insensitive() {
        let headers = vec![("content-type".to_string(), "application/json".to_string())];
        assert_eq!(
            find_content_type(&headers),
            Some("application/json".to_string())
        );
    }

    #[test]
    fn find_content_type_returns_none_when_absent() {
        let headers = vec![("Date".to_string(), "...".to_string())];
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
}
