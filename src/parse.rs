use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
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

/// CSV column indices for the canonical format: id,text,start_ms,end_ms,start_formatted,end_formatted
const COL_ID: usize = 0;
const COL_TEXT: usize = 1;

/// Parse a preprocessed CSV file. Expects columns: id,text,start_ms,end_ms,start_formatted,end_formatted (no header row).
pub fn parse_csv(path: &Path) -> crate::error::Result<Vec<Transcription>> {
    let file = File::open(path).map_err(|e| AppError::FileOpen {
        path: path.display().to_string(),
        source: e,
    })?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(BufReader::new(file));
    let mut transcriptions = Vec::new();
    let mut seen_ids = HashSet::new();

    for (line_num, result) in rdr.records().enumerate() {
        let record = result.map_err(|e| AppError::Generic(format!("line {}: {e}", line_num + 1)))?;

        let id = record
            .get(COL_ID)
            .ok_or_else(|| AppError::MissingTextField {
                line: line_num + 1,
                key: "id".to_string(),
            })?
            .to_string();

        let text = record
            .get(COL_TEXT)
            .ok_or_else(|| AppError::MissingTextField {
                line: line_num + 1,
                key: "text".to_string(),
            })?
            .to_string();

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

    const HEADER: &str = "id,text,start_ms,end_ms,start_formatted,end_formatted";

    fn write_temp_csv(rows: &[&str]) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, "{}", HEADER).unwrap();
        for row in rows {
            writeln!(f, "{}", row).unwrap();
        }
        f
    }

    #[test]
    fn parse_canonical_format() {
        let f = write_temp_csv(&[
            "a1,hello world,0,2500,00:00:00.000,00:00:02.500",
            "a2,goodbye,2500,5000,00:00:02.500,00:00:05.000",
        ]);
        let entries = parse_csv(f.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "a1");
        assert_eq!(entries[0].text, "hello world");
        assert_eq!(entries[1].id, "a2");
        assert_eq!(entries[1].text, "goodbye");
        assert_eq!(entries[0].index, 0);
        assert_eq!(entries[1].index, 1);
    }

    #[test]
    fn parse_text_with_commas() {
        let f = write_temp_csv(&[
            r#"a1,"hello, world",0,100,00:00:00.000,00:00:00.100"#,
        ]);
        let entries = parse_csv(f.path()).unwrap();
        assert_eq!(entries[0].text, "hello, world");
    }

    #[test]
    fn parse_duplicate_id_returns_err() {
        let f = write_temp_csv(&[
            "same-id,text a,0,100,00:00:00.000,00:00:00.100",
            "same-id,text b,100,200,00:00:00.100,00:00:00.200",
        ]);
        let result = parse_csv(f.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::DuplicateId { .. }));
    }

    #[test]
    fn parse_index_matches_line_number() {
        let f = write_temp_csv(&[
            "1,a,,,,", "2,b,,,,", "3,c,,,,"
        ]);
        let entries = parse_csv(f.path()).unwrap();
        assert_eq!(entries[0].index, 0);
        assert_eq!(entries[1].index, 1);
        assert_eq!(entries[2].index, 2);
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
}
