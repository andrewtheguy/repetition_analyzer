use serde_json::{Map, Value};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use uuid::Uuid;

use crate::error::AppError;
use crate::parse::{filter_matches, parse_filter, ParsedFilter};

pub struct PreprocessConfig {
    pub file: String,
    pub text_key: String,
    pub id_key: Option<String>,
    pub start_ms_key: String,
    pub end_ms_key: String,
    pub start_formatted_key: String,
    pub end_formatted_key: String,
    pub filter: Option<String>,
}

/// Canonical output field names.
const OUT_TEXT: &str = "text";
const OUT_ID: &str = "id";
const OUT_START_MS: &str = "start_ms";
const OUT_END_MS: &str = "end_ms";
const OUT_START_FMT: &str = "start_formatted";
const OUT_END_FMT: &str = "end_formatted";

fn ms_to_formatted(ms: i64) -> String {
    let total_secs = ms / 1000;
    let millis = ms % 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}

fn formatted_to_ms(s: &str) -> Option<i64> {
    // Parse HH:MM:SS.mmm
    let (hms, millis_str) = s.split_once('.')?;
    let parts: Vec<&str> = hms.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let hours: i64 = parts[0].parse().ok()?;
    let minutes: i64 = parts[1].parse().ok()?;
    let seconds: i64 = parts[2].parse().ok()?;
    let millis: i64 = millis_str.parse().ok()?;
    Some((hours * 3600 + minutes * 60 + seconds) * 1000 + millis)
}

fn process_entry(
    obj: &Value,
    config: &PreprocessConfig,
    filter: &Option<ParsedFilter>,
) -> Result<Option<Value>, String> {
    // Apply filter
    if let Some(f) = filter {
        match obj.get(&f.key) {
            Some(v) => match filter_matches(v, f) {
                Ok(true) => {}
                Ok(false) => return Ok(None),
                Err(e) => return Err(e),
            },
            None => return Ok(None),
        }
    }

    // Skip entries missing the text key
    let text = match obj.get(&config.text_key).and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return Ok(None),
    };

    // Build canonical output
    let mut out = Map::new();
    out.insert(OUT_TEXT.to_string(), Value::String(text.to_string()));

    // ID: use existing key or generate UUIDv7.
    // If --id-key is set, every entry must have a non-null value for it.
    let id = match &config.id_key {
        Some(key) => match obj.get(key) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            _ => return Err(format!("missing or null id key '{key}'")),
        },
        None => Uuid::now_v7().to_string(),
    };
    out.insert(OUT_ID.to_string(), Value::String(id));

    // Timestamps: use what's available, convert to fill gaps
    let start_ms = obj.get(&config.start_ms_key).and_then(|v| v.as_i64());
    let end_ms = obj.get(&config.end_ms_key).and_then(|v| v.as_i64());
    let start_fmt = obj
        .get(&config.start_formatted_key)
        .and_then(|v| v.as_str());
    let end_fmt = obj
        .get(&config.end_formatted_key)
        .and_then(|v| v.as_str());

    // start_ms / start_formatted
    match (start_ms, start_fmt) {
        (Some(ms), Some(fmt)) => {
            out.insert(OUT_START_MS.to_string(), Value::from(ms));
            out.insert(OUT_START_FMT.to_string(), Value::from(fmt));
        }
        (Some(ms), None) => {
            out.insert(OUT_START_MS.to_string(), Value::from(ms));
            out.insert(OUT_START_FMT.to_string(), Value::from(ms_to_formatted(ms)));
        }
        (None, Some(fmt)) => {
            if let Some(ms) = formatted_to_ms(fmt) {
                out.insert(OUT_START_MS.to_string(), Value::from(ms));
            }
            out.insert(OUT_START_FMT.to_string(), Value::from(fmt));
        }
        (None, None) => {}
    }

    // end_ms / end_formatted
    match (end_ms, end_fmt) {
        (Some(ms), Some(fmt)) => {
            out.insert(OUT_END_MS.to_string(), Value::from(ms));
            out.insert(OUT_END_FMT.to_string(), Value::from(fmt));
        }
        (Some(ms), None) => {
            out.insert(OUT_END_MS.to_string(), Value::from(ms));
            out.insert(OUT_END_FMT.to_string(), Value::from(ms_to_formatted(ms)));
        }
        (None, Some(fmt)) => {
            if let Some(ms) = formatted_to_ms(fmt) {
                out.insert(OUT_END_MS.to_string(), Value::from(ms));
            }
            out.insert(OUT_END_FMT.to_string(), Value::from(fmt));
        }
        (None, None) => {}
    }

    Ok(Some(Value::Object(out)))
}

pub fn run_preprocess(config: &PreprocessConfig) -> crate::error::Result<()> {
    let parsed_filter = parse_filter(&config.filter);
    let file = File::open(Path::new(&config.file)).map_err(|e| AppError::FileOpen {
        path: config.file.clone(),
        source: e,
    })?;
    let reader = BufReader::new(file);
    let mut stdout = std::io::BufWriter::new(std::io::stdout().lock());
    let mut count = 0usize;

    for (line_num, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| AppError::LineRead {
            line: line_num + 1,
            source: e,
        })?;
        let obj: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        match process_entry(&obj, config, &parsed_filter) {
            Ok(Some(canonical)) => {
                serde_json::to_writer(&mut stdout, &canonical)?;
                writeln!(stdout)?;
                count += 1;
            }
            Ok(None) => continue,
            Err(message) => {
                return Err(AppError::FilterMismatch {
                    line: line_num + 1,
                    message,
                })
            }
        }
    }

    drop(stdout);
    eprintln!("Wrote {count} entries");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> PreprocessConfig {
        PreprocessConfig {
            file: String::new(),
            text_key: "text".to_string(),
            id_key: None,
            start_ms_key: "start_ms".to_string(),
            end_ms_key: "end_ms".to_string(),
            start_formatted_key: "start_formatted".to_string(),
            end_formatted_key: "end_formatted".to_string(),
            filter: None,
        }
    }

    fn apply(json: &str, config: &PreprocessConfig, filter: &Option<ParsedFilter>) -> Option<Value> {
        let obj: Value = serde_json::from_str(json).unwrap();
        process_entry(&obj, config, filter).unwrap()
    }

    #[test]
    fn preprocess_filters_entries() {
        let config = default_config();
        let filter = parse_filter(&Some("type=transcript".to_string()));
        assert!(apply(r#"{"type": "meta", "text": "ignored"}"#, &config, &filter).is_none());
        let kept = apply(r#"{"type": "transcript", "text": "kept"}"#, &config, &filter);
        assert_eq!(kept.unwrap()["text"], "kept");
    }

    #[test]
    fn preprocess_generates_uuid_without_id_key() {
        let config = default_config();
        let r0 = apply(r#"{"text": "hello"}"#, &config, &None).unwrap();
        let r1 = apply(r#"{"text": "world"}"#, &config, &None).unwrap();

        let id0 = r0["id"].as_str().unwrap();
        let id1 = r1["id"].as_str().unwrap();
        assert!(Uuid::parse_str(id0).is_ok());
        assert!(Uuid::parse_str(id1).is_ok());
        assert_ne!(id0, id1);
    }

    #[test]
    fn preprocess_uses_existing_id_key() {
        let mut config = default_config();
        config.id_key = Some("uid".to_string());
        let result = apply(r#"{"text": "hello", "uid": "abc-123"}"#, &config, &None).unwrap();
        assert_eq!(result["id"], "abc-123");
    }

    #[test]
    fn preprocess_skips_missing_text_key() {
        let config = default_config();
        assert!(apply(r#"{"text": "kept"}"#, &config, &None).is_some());
        assert!(apply(r#"{"other": "no text field"}"#, &config, &None).is_none());
    }

    #[test]
    fn preprocess_outputs_canonical_fields() {
        let config = default_config();
        let result = apply(
            r#"{"text": "hi", "start_ms": 100, "end_ms": 200, "start_formatted": "0:00:00.100", "end_formatted": "0:00:00.200", "extra": "dropped"}"#,
            &config,
            &None,
        ).unwrap();
        assert_eq!(result["text"], "hi");
        assert_eq!(result["start_ms"], 100);
        assert_eq!(result["end_ms"], 200);
        assert!(result.get("extra").is_none());
    }

    #[test]
    fn preprocess_converts_ms_to_formatted() {
        let config = default_config();
        let result = apply(
            r#"{"text": "hi", "start_ms": 7552, "end_ms": 90061001}"#,
            &config,
            &None,
        ).unwrap();
        assert_eq!(result["start_formatted"], "00:00:07.552");
        assert_eq!(result["end_formatted"], "25:01:01.001");
    }

    #[test]
    fn preprocess_converts_formatted_to_ms() {
        let config = default_config();
        let result = apply(
            r#"{"text": "hi", "start_formatted": "01:30:05.250", "end_formatted": "02:00:00.000"}"#,
            &config,
            &None,
        ).unwrap();
        assert_eq!(result["start_ms"], 5405250);
        assert_eq!(result["end_ms"], 7200000);
    }

    #[test]
    fn preprocess_renames_custom_keys() {
        let config = PreprocessConfig {
            file: String::new(),
            text_key: "content".to_string(),
            id_key: Some("my_id".to_string()),
            start_ms_key: "begin".to_string(),
            end_ms_key: "finish".to_string(),
            start_formatted_key: "begin_fmt".to_string(),
            end_formatted_key: "finish_fmt".to_string(),
            filter: None,
        };
        let result = apply(
            r#"{"content": "hi", "my_id": "x1", "begin": 0, "finish": 100, "begin_fmt": "0:00:00.000", "finish_fmt": "0:00:00.100"}"#,
            &config,
            &None,
        ).unwrap();
        assert_eq!(result["text"], "hi");
        assert_eq!(result["id"], "x1");
        assert_eq!(result["start_ms"], 0);
        assert_eq!(result["end_ms"], 100);
    }

    #[test]
    fn ms_to_formatted_basic() {
        assert_eq!(ms_to_formatted(0), "00:00:00.000");
        assert_eq!(ms_to_formatted(7552), "00:00:07.552");
        assert_eq!(ms_to_formatted(3661001), "01:01:01.001");
    }

    #[test]
    fn formatted_to_ms_basic() {
        assert_eq!(formatted_to_ms("00:00:00.000"), Some(0));
        assert_eq!(formatted_to_ms("00:00:07.552"), Some(7552));
        assert_eq!(formatted_to_ms("01:01:01.001"), Some(3661001));
    }
}
