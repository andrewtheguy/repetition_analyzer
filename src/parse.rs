use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
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

/// Parse a preprocessed JSONL file. Expects canonical format with "text" and "id" fields.
pub fn parse_jsonl(path: &Path) -> crate::error::Result<Vec<Transcription>> {
    let file = File::open(path).map_err(|e| AppError::FileOpen {
        path: path.display().to_string(),
        source: e,
    })?;
    let reader = BufReader::new(file);
    let mut transcriptions = Vec::new();
    let mut seen_ids = HashSet::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| AppError::LineRead {
            line: line_num + 1,
            source: e,
        })?;
        let obj: Value = serde_json::from_str(&line).map_err(|e| AppError::InvalidJson {
            line: line_num + 1,
            source: e,
        })?;

        let text = match obj.get("text") {
            Some(Value::String(s)) => s.clone(),
            _ => {
                return Err(AppError::MissingTextField {
                    line: line_num + 1,
                    key: "text".to_string(),
                })
            }
        };

        let id = match obj.get("id") {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            _ => {
                return Err(AppError::MissingTextField {
                    line: line_num + 1,
                    key: "id".to_string(),
                })
            }
        };

        if !seen_ids.insert(id.clone()) {
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
    fn parse_canonical_format() {
        let f = write_temp_jsonl(&[
            r#"{"text": "hello world", "id": "a1"}"#,
            r#"{"text": "goodbye", "id": "a2"}"#,
        ]);
        let entries = parse_jsonl(f.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "hello world");
        assert_eq!(entries[0].id, "a1");
        assert_eq!(entries[1].text, "goodbye");
        assert_eq!(entries[1].id, "a2");
        assert_eq!(entries[0].index, 0);
        assert_eq!(entries[1].index, 1);
    }

    #[test]
    fn parse_errors_on_missing_text() {
        let f = write_temp_jsonl(&[r#"{"id": "a1", "other": "no text"}"#]);
        let result = parse_jsonl(f.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::MissingTextField { .. }));
    }

    #[test]
    fn parse_errors_on_missing_id() {
        let f = write_temp_jsonl(&[r#"{"text": "hello"}"#]);
        let result = parse_jsonl(f.path());
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
    fn parse_duplicate_id_returns_err() {
        let f = write_temp_jsonl(&[
            r#"{"text": "a", "id": "same-id"}"#,
            r#"{"text": "b", "id": "same-id"}"#,
        ]);
        let result = parse_jsonl(f.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::DuplicateId { .. }));
    }

    #[test]
    fn parse_errors_on_invalid_json() {
        let f = write_temp_jsonl(&["not json"]);
        let result = parse_jsonl(f.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::InvalidJson { .. }));
    }

    #[test]
    fn parse_index_matches_line_number() {
        let f = write_temp_jsonl(&[
            r#"{"text": "a", "id": "1"}"#,
            r#"{"text": "b", "id": "2"}"#,
            r#"{"text": "c", "id": "3"}"#,
        ]);
        let entries = parse_jsonl(f.path()).unwrap();
        assert_eq!(entries[0].index, 0);
        assert_eq!(entries[1].index, 1);
        assert_eq!(entries[2].index, 2);
    }
}
