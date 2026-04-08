use serde::Serialize;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum FilterType {
    Str,
    Bool,
    Int,
    Float,
}

#[derive(Debug, Clone)]
pub struct ParsedFilter {
    pub key: String,
    pub value: String,
    pub filter_type: FilterType,
}

/// Check whether a JSON value matches a filter using its explicit type.
pub fn filter_matches(json_val: &Value, filter: &ParsedFilter) -> bool {
    match filter.filter_type {
        FilterType::Str => matches!(json_val, Value::String(s) if s == &filter.value),
        FilterType::Bool => {
            let expected: bool = filter.value.parse().expect("invalid bool filter value");
            matches!(json_val, Value::Bool(b) if *b == expected)
        }
        FilterType::Int => {
            let expected: i64 = filter.value.parse().expect("invalid int filter value");
            json_val.as_i64() == Some(expected)
        }
        FilterType::Float => {
            let expected: f64 = filter.value.parse().expect("invalid float filter value");
            json_val.as_f64() == Some(expected)
        }
    }
}

/// Parse `--filter key=value` or `--filter key:type=value` where type is str, bool, int, float.
pub fn parse_filter(filter: &Option<String>) -> Option<ParsedFilter> {
    let f = filter.as_ref()?;
    let (key_part, value) = f
        .split_once('=')
        .expect("--filter must be in key=value or key:type=value format");
    let (key, filter_type) = match key_part.split_once(':') {
        Some((k, t)) => {
            let ft = match t {
                "str" => FilterType::Str,
                "bool" => FilterType::Bool,
                "int" => FilterType::Int,
                "float" => FilterType::Float,
                _ => panic!("unknown filter type '{t}', expected str, bool, int, or float"),
            };
            (k, ft)
        }
        None => (key_part, FilterType::Str),
    };
    Some(ParsedFilter {
        key: key.to_string(),
        value: value.to_string(),
        filter_type,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct Transcription {
    pub index: usize,
    pub id: String,
    pub text: String,
}

pub struct ParseOptions {
    pub text_key: String,
    pub id_key: Option<String>,
    pub filter: Option<ParsedFilter>,
}

pub fn parse_jsonl(path: &Path, opts: &ParseOptions) -> Vec<Transcription> {
    let file = File::open(path).expect("Failed to open JSONL file");
    let reader = BufReader::new(file);
    let mut transcriptions = Vec::new();
    let mut idx = 0;

    for (line_num, line) in reader.lines().enumerate() {
        let line = line.expect("Failed to read line");
        let obj: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Apply filter if configured
        if let Some(f) = &opts.filter {
            match obj.get(&f.key) {
                Some(v) if filter_matches(v, f) => {}
                _ => continue,
            }
        }

        // Extract text
        let text = match obj.get(&opts.text_key) {
            Some(Value::String(s)) => s.clone(),
            _ => continue,
        };

        // Extract id
        let id = if let Some(id_key) = &opts.id_key {
            match obj.get(id_key) {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Number(n)) => n.to_string(),
                _ => (line_num + 1).to_string(),
            }
        } else {
            (line_num + 1).to_string()
        };

        transcriptions.push(Transcription {
            index: idx,
            id,
            text,
        });
        idx += 1;
    }

    transcriptions
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_jsonl(lines: &[&str]) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(f, "{}", line).unwrap();
        }
        f
    }

    #[test]
    fn parse_default_text_key() {
        let f = write_temp_jsonl(&[r#"{"text": "hello world"}"#, r#"{"text": "goodbye"}"#]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: None,
            filter: None,
        };
        let entries = parse_jsonl(f.path(), &opts);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "hello world");
        assert_eq!(entries[1].text, "goodbye");
        assert_eq!(entries[0].index, 0);
        assert_eq!(entries[1].index, 1);
    }

    #[test]
    fn parse_custom_text_key() {
        let f = write_temp_jsonl(&[r#"{"content": "foo bar"}"#]);
        let opts = ParseOptions {
            text_key: "content".to_string(),
            id_key: None,
            filter: None,
        };
        let entries = parse_jsonl(f.path(), &opts);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "foo bar");
    }

    #[test]
    fn parse_skips_missing_text_key() {
        let f = write_temp_jsonl(&[r#"{"text": "kept"}"#, r#"{"other": "skipped"}"#]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: None,
            filter: None,
        };
        let entries = parse_jsonl(f.path(), &opts);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "kept");
    }

    #[test]
    fn parse_with_filter() {
        let f = write_temp_jsonl(&[
            r#"{"type": "meta", "text": "ignored"}"#,
            r#"{"type": "transcription", "text": "kept"}"#,
            r#"{"type": "transcription", "text": "also kept"}"#,
        ]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: None,
            filter: Some(ParsedFilter {
                key: "type".to_string(),
                value: "transcription".to_string(),
                filter_type: FilterType::Str,
            }),
        };
        let entries = parse_jsonl(f.path(), &opts);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "kept");
        assert_eq!(entries[1].text, "also kept");
    }

    #[test]
    fn parse_id_from_line_number() {
        let f = write_temp_jsonl(&[r#"{"other": "skip"}"#, r#"{"text": "hello"}"#]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: None,
            filter: None,
        };
        let entries = parse_jsonl(f.path(), &opts);
        assert_eq!(entries[0].id, "2"); // line 2 (1-indexed)
    }

    #[test]
    fn parse_custom_id_key() {
        let f = write_temp_jsonl(&[r#"{"text": "hi", "uid": "abc-123"}"#]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: Some("uid".to_string()),
            filter: None,
        };
        let entries = parse_jsonl(f.path(), &opts);
        assert_eq!(entries[0].id, "abc-123");
    }

    #[test]
    fn parse_skips_invalid_json() {
        let f = write_temp_jsonl(&["not json", r#"{"text": "valid"}"#]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: None,
            filter: None,
        };
        let entries = parse_jsonl(f.path(), &opts);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "valid");
    }
}
