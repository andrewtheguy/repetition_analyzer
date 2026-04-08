use serde::Serialize;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::error::AppError;

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
}

/// Parse a preprocessed JSONL file. Every line must be valid JSON with the text key present.
pub fn parse_jsonl(path: &Path, opts: &ParseOptions) -> crate::error::Result<Vec<Transcription>> {
    let file = File::open(path).map_err(|e| AppError::FileOpen {
        path: path.display().to_string(),
        source: e,
    })?;
    let reader = BufReader::new(file);
    let mut transcriptions = Vec::new();
    let mut seen_ids: Option<std::collections::HashSet<String>> =
        if opts.id_key.is_some() { Some(std::collections::HashSet::new()) } else { None };

    for (line_num, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| AppError::LineRead {
            line: line_num + 1,
            source: e,
        })?;
        let obj: Value = serde_json::from_str(&line).map_err(|e| AppError::InvalidJson {
            line: line_num + 1,
            source: e,
        })?;

        let text = match obj.get(&opts.text_key) {
            Some(Value::String(s)) => s.clone(),
            _ => {
                return Err(AppError::MissingTextField {
                    line: line_num + 1,
                    key: opts.text_key.clone(),
                })
            }
        };

        let id = if let Some(id_key) = &opts.id_key {
            match obj.get(id_key) {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Number(n)) => n.to_string(),
                _ => (line_num + 1).to_string(),
            }
        } else {
            (line_num + 1).to_string()
        };

        if let Some(seen) = &mut seen_ids
            && !seen.insert(id.clone())
        {
            return Err(AppError::DuplicateId {
                line: line_num + 1,
                id,
            });
        }

        transcriptions.push(Transcription {
            index: line_num,
            id,
            text,
        });
    }

    Ok(transcriptions)
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
        };
        let entries = parse_jsonl(f.path(), &opts).unwrap();
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
        };
        let entries = parse_jsonl(f.path(), &opts).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "foo bar");
    }

    #[test]
    fn parse_errors_on_missing_text_key() {
        let f = write_temp_jsonl(&[r#"{"text": "kept"}"#, r#"{"other": "no text"}"#]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: None,
        };
        let result = parse_jsonl(f.path(), &opts);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::MissingTextField { .. }));
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
    fn parse_id_from_line_number() {
        let f = write_temp_jsonl(&[r#"{"text": "first"}"#, r#"{"text": "hello"}"#]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: None,
        };
        let entries = parse_jsonl(f.path(), &opts).unwrap();
        assert_eq!(entries[0].id, "1");
        assert_eq!(entries[1].id, "2");
    }

    #[test]
    fn parse_custom_id_key() {
        let f = write_temp_jsonl(&[r#"{"text": "hi", "uid": "abc-123"}"#]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: Some("uid".to_string()),
        };
        let entries = parse_jsonl(f.path(), &opts).unwrap();
        assert_eq!(entries[0].id, "abc-123");
    }

    #[test]
    fn parse_unique_ids_accepted() {
        let f = write_temp_jsonl(&[
            r#"{"text": "a", "uid": "id-1"}"#,
            r#"{"text": "b", "uid": "id-2"}"#,
            r#"{"text": "c", "uid": "id-3"}"#,
        ]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: Some("uid".to_string()),
        };
        let entries = parse_jsonl(f.path(), &opts).unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn parse_no_uniqueness_check_without_id_key() {
        let f = write_temp_jsonl(&[
            r#"{"text": "a"}"#,
            r#"{"text": "b"}"#,
        ]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: None,
        };
        let entries = parse_jsonl(f.path(), &opts).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn parse_duplicate_id_returns_err() {
        let f = write_temp_jsonl(&[
            r#"{"text": "a", "uid": "same-id"}"#,
            r#"{"text": "b", "uid": "same-id"}"#,
        ]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: Some("uid".to_string()),
        };
        let result = parse_jsonl(f.path(), &opts);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AppError::DuplicateId { .. }
        ));
    }

    #[test]
    fn parse_errors_on_invalid_json() {
        let f = write_temp_jsonl(&["not json", r#"{"text": "valid"}"#]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: None,
        };
        let result = parse_jsonl(f.path(), &opts);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::InvalidJson { .. }));
    }

    #[test]
    fn parse_index_matches_line_number() {
        let f = write_temp_jsonl(&[
            r#"{"text": "a"}"#,
            r#"{"text": "b"}"#,
            r#"{"text": "c"}"#,
        ]);
        let opts = ParseOptions {
            text_key: "text".to_string(),
            id_key: None,
        };
        let entries = parse_jsonl(f.path(), &opts).unwrap();
        assert_eq!(entries[0].index, 0);
        assert_eq!(entries[1].index, 1);
        assert_eq!(entries[2].index, 2);
    }
}
