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

fn json_type_name(val: &Value) -> &'static str {
    match val {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Check whether a JSON value matches a filter using its explicit type.
/// Returns Ok(true) on match, Ok(false) for null/missing, Err on type mismatch.
pub fn filter_matches(json_val: &Value, filter: &ParsedFilter) -> Result<bool, String> {
    if json_val.is_null() {
        return Ok(false);
    }
    match filter.filter_type {
        FilterType::Str => match json_val {
            Value::String(s) => Ok(s == &filter.value),
            _ => Err(format!(
                "filter type mismatch for key '{}': expected string, got {}",
                filter.key,
                json_type_name(json_val)
            )),
        },
        FilterType::Bool => {
            let expected: bool = filter.value.parse().expect("invalid bool filter value");
            match json_val {
                Value::Bool(b) => Ok(*b == expected),
                _ => Err(format!(
                    "filter type mismatch for key '{}': expected bool, got {}",
                    filter.key,
                    json_type_name(json_val)
                )),
            }
        }
        FilterType::Int => {
            let expected: i64 = filter.value.parse().expect("invalid int filter value");
            match json_val {
                Value::Number(n) => Ok(n.as_i64() == Some(expected)),
                _ => Err(format!(
                    "filter type mismatch for key '{}': expected number, got {}",
                    filter.key,
                    json_type_name(json_val)
                )),
            }
        }
        FilterType::Float => {
            let expected: f64 = filter.value.parse().expect("invalid float filter value");
            match json_val {
                Value::Number(n) => Ok(n.as_f64() == Some(expected)),
                _ => Err(format!(
                    "filter type mismatch for key '{}': expected number, got {}",
                    filter.key,
                    json_type_name(json_val)
                )),
            }
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
                Some(v) => match filter_matches(v, f) {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(e) => {
                        eprintln!("Error: line {}: {e}", line_num + 1);
                        std::process::exit(1);
                    }
                },
                None => continue,
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
    fn parse_with_bool_filter() {
        let f = write_temp_jsonl(&[
            r#"{"active": true, "text": "yes"}"#,
            r#"{"active": false, "text": "no"}"#,
            r#"{"active": true, "text": "also yes"}"#,
        ]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: None,
            filter: Some(ParsedFilter {
                key: "active".to_string(),
                value: "true".to_string(),
                filter_type: FilterType::Bool,
            }),
        };
        let entries = parse_jsonl(f.path(), &opts);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "yes");
        assert_eq!(entries[1].text, "also yes");
    }

    #[test]
    fn parse_with_int_filter() {
        let f = write_temp_jsonl(&[
            r#"{"status": 1, "text": "one"}"#,
            r#"{"status": 2, "text": "two"}"#,
            r#"{"status": 1, "text": "another one"}"#,
        ]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: None,
            filter: Some(ParsedFilter {
                key: "status".to_string(),
                value: "1".to_string(),
                filter_type: FilterType::Int,
            }),
        };
        let entries = parse_jsonl(f.path(), &opts);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "one");
        assert_eq!(entries[1].text, "another one");
    }

    #[test]
    fn parse_with_float_filter() {
        let f = write_temp_jsonl(&[
            r#"{"score": 0.5, "text": "half"}"#,
            r#"{"score": 1.0, "text": "full"}"#,
            r#"{"score": 0.5, "text": "also half"}"#,
        ]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: None,
            filter: Some(ParsedFilter {
                key: "score".to_string(),
                value: "0.5".to_string(),
                filter_type: FilterType::Float,
            }),
        };
        let entries = parse_jsonl(f.path(), &opts);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "half");
        assert_eq!(entries[1].text, "also half");
    }

    #[test]
    fn filter_matches_type_mismatch_returns_err() {
        let filter = ParsedFilter {
            key: "status".to_string(),
            value: "1".to_string(),
            filter_type: FilterType::Int,
        };
        let json_val: Value = Value::String("1".to_string());
        let result = filter_matches(&json_val, &filter);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("filter type mismatch"));
    }

    #[test]
    fn parse_filter_null_value_skips() {
        let f = write_temp_jsonl(&[
            r#"{"status": null, "text": "null status"}"#,
            r#"{"status": 1, "text": "int one"}"#,
        ]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: None,
            filter: Some(ParsedFilter {
                key: "status".to_string(),
                value: "1".to_string(),
                filter_type: FilterType::Int,
            }),
        };
        let entries = parse_jsonl(f.path(), &opts);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "int one");
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
