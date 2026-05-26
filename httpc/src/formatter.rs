use std::fmt::Write as _;
use std::io::IsTerminal;

use anstyle::{AnsiColor, Color, Effects, Style};
use quick_xml::events::{BytesEnd, BytesStart, Event};
use quick_xml::reader::Reader;

use crate::client::Response;
use crate::parser::Request;

#[derive(Clone, Copy)]
pub struct Theme {
    method_get: Style,
    method_post: Style,
    method_put: Style,
    method_delete: Style,
    method_patch: Style,
    method_other: Style,
    header_name: Style,
    status_2xx: Style,
    status_3xx: Style,
    status_4xx: Style,
    status_5xx: Style,
    dim: Style,
    json_key: Style,
    json_string: Style,
    json_number: Style,
    json_bool: Style,
    json_null: Style,
    json_punct: Style,
    xml_tag: Style,
    xml_attr_name: Style,
    xml_attr_value: Style,
    xml_punct: Style,
    xml_comment: Style,
    xml_cdata: Style,
}

impl Theme {
    pub fn auto() -> Self {
        if std::env::var_os("NO_COLOR").is_some() || !std::io::stdout().is_terminal() {
            Self::plain()
        } else {
            Self::colored()
        }
    }

    fn plain() -> Self {
        let s = Style::new();
        Self {
            method_get: s,
            method_post: s,
            method_put: s,
            method_delete: s,
            method_patch: s,
            method_other: s,
            header_name: s,
            status_2xx: s,
            status_3xx: s,
            status_4xx: s,
            status_5xx: s,
            dim: s,
            json_key: s,
            json_string: s,
            json_number: s,
            json_bool: s,
            json_null: s,
            json_punct: s,
            xml_tag: s,
            xml_attr_name: s,
            xml_attr_value: s,
            xml_punct: s,
            xml_comment: s,
            xml_cdata: s,
        }
    }

    fn colored() -> Self {
        let fg = |c: AnsiColor| Style::new().fg_color(Some(Color::Ansi(c)));
        let bold_fg = |c: AnsiColor| fg(c).effects(Effects::BOLD);
        let dim = Style::new().effects(Effects::DIMMED);
        Self {
            method_get: bold_fg(AnsiColor::Green),
            method_post: bold_fg(AnsiColor::Yellow),
            method_put: bold_fg(AnsiColor::Blue),
            method_delete: bold_fg(AnsiColor::Red),
            method_patch: bold_fg(AnsiColor::Magenta),
            method_other: bold_fg(AnsiColor::Cyan),
            header_name: fg(AnsiColor::Cyan),
            status_2xx: bold_fg(AnsiColor::Green),
            status_3xx: bold_fg(AnsiColor::Cyan),
            status_4xx: bold_fg(AnsiColor::Yellow),
            status_5xx: bold_fg(AnsiColor::Red),
            dim,
            json_key: fg(AnsiColor::Cyan),
            json_string: fg(AnsiColor::Green),
            json_number: fg(AnsiColor::Yellow),
            json_bool: fg(AnsiColor::Magenta),
            json_null: fg(AnsiColor::Red),
            json_punct: dim,
            xml_tag: fg(AnsiColor::Cyan),
            xml_attr_name: fg(AnsiColor::Yellow),
            xml_attr_value: fg(AnsiColor::Green),
            xml_punct: dim,
            xml_comment: dim,
            xml_cdata: dim,
        }
    }

    fn method_style(&self, method: &str) -> Style {
        match method.to_ascii_uppercase().as_str() {
            "GET" => self.method_get,
            "POST" => self.method_post,
            "PUT" => self.method_put,
            "DELETE" => self.method_delete,
            "PATCH" => self.method_patch,
            _ => self.method_other,
        }
    }

    fn status_style(&self, code: u16) -> Style {
        match code {
            200..=299 => self.status_2xx,
            300..=399 => self.status_3xx,
            400..=499 => self.status_4xx,
            500..=599 => self.status_5xx,
            _ => Style::new(),
        }
    }
}

pub fn print_request(req: &Request, theme: &Theme) {
    let m = theme.method_style(&req.method);
    println!("{m}{}{m:#} {}", req.method, req.url);
    let h = theme.header_name;
    for (name, value) in &req.headers {
        println!("{h}{name}{h:#}: {value}");
    }
    if !req.body.is_empty() {
        println!();
        let content_type = find_content_type(&req.headers);
        let pretty = pretty_body(&req.body, content_type.as_deref(), theme);
        println!("{pretty}");
    }
    println!();
}

pub fn print_response(resp: &Response, theme: &Theme) {
    let s = theme.status_style(resp.status_code);
    let d = theme.dim;
    if resp.reason_phrase.is_empty() {
        println!("{d}{}{d:#} {s}{}{s:#}", resp.http_version, resp.status_code);
    } else {
        println!(
            "{d}{}{d:#} {s}{} {}{s:#}",
            resp.http_version, resp.status_code, resp.reason_phrase
        );
    }
    let h = theme.header_name;
    for (name, value) in &resp.headers {
        println!("{h}{name}{h:#}: {value}");
    }
    println!();

    let body_str = String::from_utf8_lossy(&resp.body);
    let content_type = find_content_type(&resp.headers);
    let pretty = pretty_body(&body_str, content_type.as_deref(), theme);
    println!("{pretty}");
    println!();

    println!(
        "{d}Response code: {}; Time: {}ms; Content length: {} bytes{d:#}",
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

fn pretty_body(body: &str, content_type: Option<&str>, theme: &Theme) -> String {
    match content_type {
        Some(ct) if is_json(ct) => pretty_json(body, theme),
        Some(ct) if is_xml(ct) => pretty_xml(body, theme),
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

fn pretty_json(input: &str, theme: &Theme) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(input) else {
        return input.to_string();
    };
    let mut out = String::new();
    write_json_value(&mut out, &value, theme, 0);
    out
}

fn write_json_value(out: &mut String, value: &serde_json::Value, theme: &Theme, depth: usize) {
    use serde_json::Value;
    match value {
        Value::Null => {
            let s = theme.json_null;
            let _ = write!(out, "{s}null{s:#}");
        }
        Value::Bool(b) => {
            let s = theme.json_bool;
            let _ = write!(out, "{s}{b}{s:#}");
        }
        Value::Number(n) => {
            let s = theme.json_number;
            let _ = write!(out, "{s}{n}{s:#}");
        }
        Value::String(s_val) => {
            let escaped = serde_json::to_string(s_val).unwrap_or_else(|_| format!("\"{s_val}\""));
            let s = theme.json_string;
            let _ = write!(out, "{s}{escaped}{s:#}");
        }
        Value::Array(arr) => {
            let p = theme.json_punct;
            if arr.is_empty() {
                let _ = write!(out, "{p}[]{p:#}");
                return;
            }
            let _ = write!(out, "{p}[{p:#}");
            out.push('\n');
            for (i, item) in arr.iter().enumerate() {
                write_indent(out, depth + 1);
                write_json_value(out, item, theme, depth + 1);
                if i + 1 < arr.len() {
                    let _ = write!(out, "{p},{p:#}");
                }
                out.push('\n');
            }
            write_indent(out, depth);
            let _ = write!(out, "{p}]{p:#}");
        }
        Value::Object(obj) => {
            let p = theme.json_punct;
            if obj.is_empty() {
                let _ = write!(out, "{p}{{}}{p:#}");
                return;
            }
            let _ = write!(out, "{p}{{{p:#}");
            out.push('\n');
            for (i, (k, v)) in obj.iter().enumerate() {
                write_indent(out, depth + 1);
                let key_escaped = serde_json::to_string(k).unwrap_or_else(|_| format!("\"{k}\""));
                let key_s = theme.json_key;
                let _ = write!(out, "{key_s}{key_escaped}{key_s:#}{p}:{p:#} ");
                write_json_value(out, v, theme, depth + 1);
                if i + 1 < obj.len() {
                    let _ = write!(out, "{p},{p:#}");
                }
                out.push('\n');
            }
            write_indent(out, depth);
            let _ = write!(out, "{p}}}{p:#}");
        }
    }
}

fn write_indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn pretty_xml(input: &str, theme: &Theme) -> String {
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);

    let mut out = String::new();
    let mut depth: usize = 0;
    let mut last_was_text = false;
    let mut is_first = true;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(Event::Decl(d)) => {
                begin_structural(&mut out, depth, last_was_text, is_first);
                let p = theme.xml_punct;
                let _ = write!(&mut out, "{p}<?");
                if let Ok(s) = std::str::from_utf8(d.as_ref()) {
                    out.push_str(s);
                }
                let _ = write!(&mut out, "?>{p:#}");
                is_first = false;
                last_was_text = false;
            }
            Ok(Event::Start(e)) => {
                begin_structural(&mut out, depth, last_was_text, is_first);
                write_open_tag(&mut out, &e, theme);
                depth += 1;
                is_first = false;
                last_was_text = false;
            }
            Ok(Event::End(e)) => {
                depth = depth.saturating_sub(1);
                if !last_was_text {
                    if !is_first {
                        out.push('\n');
                    }
                    write_indent(&mut out, depth);
                }
                write_close_tag(&mut out, &e, theme);
                is_first = false;
                last_was_text = false;
            }
            Ok(Event::Empty(e)) => {
                begin_structural(&mut out, depth, last_was_text, is_first);
                write_empty_tag(&mut out, &e, theme);
                is_first = false;
                last_was_text = false;
            }
            Ok(Event::Text(t)) => {
                if let Ok(s) = std::str::from_utf8(t.as_ref()) {
                    out.push_str(s);
                    last_was_text = true;
                }
            }
            Ok(Event::CData(c)) => {
                let p = theme.xml_cdata;
                let _ = write!(&mut out, "{p}<![CDATA[");
                if let Ok(s) = std::str::from_utf8(c.as_ref()) {
                    out.push_str(s);
                }
                let _ = write!(&mut out, "]]>{p:#}");
                is_first = false;
                last_was_text = true;
            }
            Ok(Event::Comment(c)) => {
                begin_structural(&mut out, depth, last_was_text, is_first);
                let cm = theme.xml_comment;
                let _ = write!(&mut out, "{cm}<!--");
                if let Ok(s) = std::str::from_utf8(c.as_ref()) {
                    out.push_str(s);
                }
                let _ = write!(&mut out, "-->{cm:#}");
                is_first = false;
                last_was_text = false;
            }
            Ok(_) => {}
            Err(_) => return input.to_string(),
        }
        buf.clear();
    }

    out
}

fn begin_structural(out: &mut String, depth: usize, last_was_text: bool, is_first: bool) {
    if last_was_text {
        return;
    }
    if !is_first {
        out.push('\n');
    }
    write_indent(out, depth);
}

fn write_open_tag(out: &mut String, e: &BytesStart, theme: &Theme) {
    let p = theme.xml_punct;
    let t = theme.xml_tag;
    let name_bytes = e.name();
    let name = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");
    let _ = write!(out, "{p}<{p:#}{t}{name}{t:#}");
    write_attributes(out, e, theme);
    let _ = write!(out, "{p}>{p:#}");
}

fn write_close_tag(out: &mut String, e: &BytesEnd, theme: &Theme) {
    let p = theme.xml_punct;
    let t = theme.xml_tag;
    let name_bytes = e.name();
    let name = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");
    let _ = write!(out, "{p}</{p:#}{t}{name}{t:#}{p}>{p:#}");
}

fn write_empty_tag(out: &mut String, e: &BytesStart, theme: &Theme) {
    let p = theme.xml_punct;
    let t = theme.xml_tag;
    let name_bytes = e.name();
    let name = std::str::from_utf8(name_bytes.as_ref()).unwrap_or("");
    let _ = write!(out, "{p}<{p:#}{t}{name}{t:#}");
    write_attributes(out, e, theme);
    let _ = write!(out, "{p}/>{p:#}");
}

fn write_attributes(out: &mut String, e: &BytesStart, theme: &Theme) {
    let p = theme.xml_punct;
    let an = theme.xml_attr_name;
    let av = theme.xml_attr_value;
    for attr in e.attributes().flatten() {
        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
        let val = std::str::from_utf8(&attr.value).unwrap_or("");
        let _ = write!(out, " {an}{key}{an:#}{p}=\"{p:#}{av}{val}{av:#}{p}\"{p:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain() -> Theme {
        Theme::plain()
    }

    fn colored() -> Theme {
        Theme::colored()
    }

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
        let output = pretty_json(input, &plain());
        assert!(output.contains('\n'));
        assert!(output.contains("  "));
    }

    #[test]
    fn pretty_json_returns_input_on_invalid() {
        let input = "not json";
        assert_eq!(pretty_json(input, &plain()), "not json");
        assert_eq!(pretty_json(input, &colored()), "not json");
    }

    #[test]
    fn pretty_json_plain_has_no_escape_codes() {
        let input = r#"{"a":1,"b":"hi","c":null,"d":true}"#;
        let output = pretty_json(input, &plain());
        assert!(!output.contains('\u{1b}'));
    }

    #[test]
    fn pretty_json_colored_emits_escape_codes() {
        let input = r#"{"a":1}"#;
        let output = pretty_json(input, &colored());
        assert!(output.contains('\u{1b}'));
    }

    #[test]
    fn pretty_json_renders_all_value_kinds() {
        let input = r#"{"s":"x","n":42,"b":true,"z":null,"a":[],"o":{}}"#;
        let output = pretty_json(input, &plain());
        assert!(output.contains("\"s\""));
        assert!(output.contains("\"x\""));
        assert!(output.contains("42"));
        assert!(output.contains("true"));
        assert!(output.contains("null"));
        assert!(output.contains("[]"));
        assert!(output.contains("{}"));
    }

    #[test]
    fn pretty_xml_indents_compact_input() {
        let input = "<root><a>1</a><b>2</b></root>";
        let output = pretty_xml(input, &plain());
        assert!(output.contains('\n'));
        assert!(output.contains("  "));
    }

    #[test]
    fn pretty_xml_returns_input_on_invalid() {
        let input = "<unclosed";
        let output = pretty_xml(input, &plain());
        assert_eq!(output, "<unclosed");
    }

    #[test]
    fn pretty_xml_plain_has_no_escape_codes() {
        let input = r#"<root attr="v"><a>1</a></root>"#;
        let output = pretty_xml(input, &plain());
        assert!(!output.contains('\u{1b}'));
    }

    #[test]
    fn pretty_xml_colored_emits_escape_codes() {
        let input = "<root><a>1</a></root>";
        let output = pretty_xml(input, &colored());
        assert!(output.contains('\u{1b}'));
    }

    #[test]
    fn pretty_xml_preserves_attributes_and_text() {
        let input = r#"<root attr="v"><a>hello</a></root>"#;
        let output = pretty_xml(input, &plain());
        assert!(output.contains("attr"));
        assert!(output.contains("v"));
        assert!(output.contains("hello"));
    }
}
