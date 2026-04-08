use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
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

pub fn ms_to_formatted(ms: i64) -> String {
    let total_secs = ms / 1000;
    let millis = ms % 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}

pub fn formatted_to_ms(s: &str) -> Option<i64> {
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

/// Canonical row produced by preprocessing.
#[derive(Debug)]
struct CanonicalRow {
    id: String,
    text: String,
    start_ms: String,
    end_ms: String,
    start_formatted: String,
    end_formatted: String,
}

fn process_entry(
    obj: &Value,
    config: &PreprocessConfig,
    filter: &Option<ParsedFilter>,
) -> Result<Option<CanonicalRow>, String> {
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
        Some(s) => s.to_string(),
        None => return Ok(None),
    };

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

    // Timestamps: use what's available, convert to fill gaps
    let start_ms_val = obj.get(&config.start_ms_key).and_then(|v| v.as_i64());
    let end_ms_val = obj.get(&config.end_ms_key).and_then(|v| v.as_i64());
    let start_fmt_val = obj
        .get(&config.start_formatted_key)
        .and_then(|v| v.as_str());
    let end_fmt_val = obj
        .get(&config.end_formatted_key)
        .and_then(|v| v.as_str());

    let (start_ms, start_formatted) = match (start_ms_val, start_fmt_val) {
        (Some(ms), Some(fmt)) => (ms.to_string(), fmt.to_string()),
        (Some(ms), None) => (ms.to_string(), ms_to_formatted(ms)),
        (None, Some(fmt)) => {
            let ms = formatted_to_ms(fmt)
                .ok_or_else(|| format!("invalid start timestamp format: '{fmt}'"))?;
            (ms.to_string(), fmt.to_string())
        }
        (None, None) => {
            return Err(format!(
                "missing start timestamp (expected '{}' or '{}')",
                config.start_ms_key, config.start_formatted_key
            ));
        }
    };

    let (end_ms, end_formatted) = match (end_ms_val, end_fmt_val) {
        (Some(ms), Some(fmt)) => (ms.to_string(), fmt.to_string()),
        (Some(ms), None) => (ms.to_string(), ms_to_formatted(ms)),
        (None, Some(fmt)) => {
            let ms = formatted_to_ms(fmt)
                .ok_or_else(|| format!("invalid end timestamp format: '{fmt}'"))?;
            (ms.to_string(), fmt.to_string())
        }
        (None, None) => {
            return Err(format!(
                "missing end timestamp (expected '{}' or '{}')",
                config.end_ms_key, config.end_formatted_key
            ));
        }
    };

    Ok(Some(CanonicalRow {
        id,
        text,
        start_ms,
        end_ms,
        start_formatted,
        end_formatted,
    }))
}

pub fn run_preprocess(config: &PreprocessConfig) -> crate::error::Result<()> {
    let parsed_filter = parse_filter(&config.filter);
    let file = File::open(Path::new(&config.file)).map_err(|e| AppError::FileOpen {
        path: config.file.clone(),
        source: e,
    })?;
    let reader = BufReader::new(file);
    let stdout = std::io::BufWriter::new(std::io::stdout().lock());
    let mut wtr = csv::Writer::from_writer(stdout);
    wtr.write_record(["id", "text", "start_ms", "end_ms", "start_formatted", "end_formatted"])?;
    let mut count = 0usize;

    for (line_num, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| AppError::LineRead {
            line: line_num + 1,
            source: e,
        })?;
        let obj: Value = serde_json::from_str(&line).map_err(|e| AppError::Generic(
            format!("line {}: invalid JSON: {e}", line_num + 1),
        ))?;

        match process_entry(&obj, config, &parsed_filter) {
            Ok(Some(row)) => {
                wtr.write_record([
                    &row.id,
                    &row.text,
                    &row.start_ms,
                    &row.end_ms,
                    &row.start_formatted,
                    &row.end_formatted,
                ])?;
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

    wtr.flush()?;
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

    fn apply(json: &str, config: &PreprocessConfig, filter: &Option<ParsedFilter>) -> Option<CanonicalRow> {
        let obj: Value = serde_json::from_str(json).unwrap();
        process_entry(&obj, config, filter).unwrap()
    }

    #[test]
    fn preprocess_filters_entries() {
        let config = default_config();
        let filter = parse_filter(&Some("type=transcript".to_string()));
        assert!(apply(r#"{"type": "meta", "text": "ignored", "start_ms": 0, "end_ms": 100}"#, &config, &filter).is_none());
        let kept = apply(r#"{"type": "transcript", "text": "kept", "start_ms": 0, "end_ms": 100}"#, &config, &filter);
        assert_eq!(kept.unwrap().text, "kept");
    }

    #[test]
    fn preprocess_generates_uuid_without_id_key() {
        let config = default_config();
        let r0 = apply(r#"{"text": "hello", "start_ms": 0, "end_ms": 100}"#, &config, &None).unwrap();
        let r1 = apply(r#"{"text": "world", "start_ms": 100, "end_ms": 200}"#, &config, &None).unwrap();
        assert!(Uuid::parse_str(&r0.id).is_ok());
        assert!(Uuid::parse_str(&r1.id).is_ok());
        assert_ne!(r0.id, r1.id);
    }

    #[test]
    fn preprocess_uses_existing_id_key() {
        let mut config = default_config();
        config.id_key = Some("uid".to_string());
        let result = apply(r#"{"text": "hello", "uid": "abc-123", "start_ms": 0, "end_ms": 100}"#, &config, &None).unwrap();
        assert_eq!(result.id, "abc-123");
    }

    #[test]
    fn preprocess_skips_missing_text_key() {
        let config = default_config();
        assert!(apply(r#"{"text": "kept", "start_ms": 0, "end_ms": 100}"#, &config, &None).is_some());
        assert!(apply(r#"{"other": "no text field", "start_ms": 0, "end_ms": 100}"#, &config, &None).is_none());
    }

    #[test]
    fn preprocess_converts_ms_to_formatted() {
        let config = default_config();
        let result = apply(
            r#"{"text": "hi", "start_ms": 7552, "end_ms": 90061001}"#,
            &config,
            &None,
        ).unwrap();
        assert_eq!(result.start_formatted, "00:00:07.552");
        assert_eq!(result.end_formatted, "25:01:01.001");
    }

    #[test]
    fn preprocess_converts_formatted_to_ms() {
        let config = default_config();
        let result = apply(
            r#"{"text": "hi", "start_formatted": "01:30:05.250", "end_formatted": "02:00:00.000"}"#,
            &config,
            &None,
        ).unwrap();
        assert_eq!(result.start_ms, "5405250");
        assert_eq!(result.end_ms, "7200000");
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
        assert_eq!(result.text, "hi");
        assert_eq!(result.id, "x1");
        assert_eq!(result.start_ms, "0");
        assert_eq!(result.end_ms, "100");
    }

    fn apply_err(json: &str, config: &PreprocessConfig, filter: &Option<ParsedFilter>) -> String {
        let obj: Value = serde_json::from_str(json).unwrap();
        process_entry(&obj, config, filter).unwrap_err()
    }

    #[test]
    fn preprocess_errors_on_missing_start_timestamp() {
        let config = default_config();
        let err = apply_err(r#"{"text": "hi", "end_ms": 100}"#, &config, &None);
        assert!(err.contains("missing start timestamp"), "got: {err}");
    }

    #[test]
    fn preprocess_errors_on_missing_end_timestamp() {
        let config = default_config();
        let err = apply_err(r#"{"text": "hi", "start_ms": 0}"#, &config, &None);
        assert!(err.contains("missing end timestamp"), "got: {err}");
    }

    #[test]
    fn preprocess_errors_on_invalid_formatted_timestamp() {
        let config = default_config();
        let err = apply_err(
            r#"{"text": "hi", "start_formatted": "bad", "end_formatted": "00:00:01.000"}"#,
            &config,
            &None,
        );
        assert!(err.contains("invalid start timestamp format"), "got: {err}");
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
