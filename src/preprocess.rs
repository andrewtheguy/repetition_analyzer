use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use uuid::Uuid;

use crate::error::AppError;
use crate::parse::{filter_matches, parse_filter, ParsedFilter};

pub struct PreprocessConfig {
    pub file: String,
    pub text_key: String,
    pub filter: Option<String>,
    pub new_id_key: Option<String>,
}

fn process_entry(
    obj: &mut Value,
    text_key: &str,
    filter: &Option<ParsedFilter>,
    new_id_key: &Option<String>,
) -> Result<bool, String> {
    // Apply filter
    if let Some(f) = filter {
        match obj.get(&f.key) {
            Some(v) => match filter_matches(v, f) {
                Ok(true) => {}
                Ok(false) => return Ok(false),
                Err(e) => return Err(e),
            },
            None => return Ok(false),
        }
    }

    // Skip entries missing the text key
    if obj.get(text_key).and_then(|v| v.as_str()).is_none() {
        return Ok(false);
    }

    // Insert UUID if requested
    if let Some(key) = new_id_key
        && let Some(map) = obj.as_object_mut()
    {
        map.insert(key.clone(), Value::String(Uuid::now_v7().to_string()));
    }

    Ok(true)
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
        let mut obj: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        match process_entry(&mut obj, &config.text_key, &parsed_filter, &config.new_id_key) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(message) => {
                return Err(AppError::FilterMismatch {
                    line: line_num + 1,
                    message,
                })
            }
        }

        serde_json::to_writer(&mut stdout, &obj)?;
        writeln!(stdout)?;
        count += 1;
    }

    drop(stdout);
    eprintln!("Wrote {count} entries");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(json: &str, text_key: &str, filter: &Option<ParsedFilter>, new_id_key: &Option<String>) -> Option<Value> {
        let mut obj: Value = serde_json::from_str(json).unwrap();
        if process_entry(&mut obj, text_key, filter, new_id_key).unwrap() {
            Some(obj)
        } else {
            None
        }
    }

    #[test]
    fn preprocess_filters_entries() {
        let filter = parse_filter(&Some("type=transcription".to_string()));
        assert!(apply(r#"{"type": "meta", "text": "ignored"}"#, "text", &filter, &None).is_none());
        let kept = apply(r#"{"type": "transcription", "text": "kept"}"#, "text", &filter, &None);
        assert_eq!(kept.unwrap()["text"], "kept");
    }

    #[test]
    fn preprocess_inserts_uuid() {
        let new_id_key = Some("uuid_id".to_string());
        let r0 = apply(r#"{"text": "hello"}"#, "text", &None, &new_id_key).unwrap();
        let r1 = apply(r#"{"text": "world"}"#, "text", &None, &new_id_key).unwrap();

        let id0 = r0["uuid_id"].as_str().unwrap();
        let id1 = r1["uuid_id"].as_str().unwrap();
        assert!(Uuid::parse_str(id0).is_ok());
        assert!(Uuid::parse_str(id1).is_ok());
        assert_ne!(id0, id1);
    }

    #[test]
    fn preprocess_no_uuid_without_new_id_key() {
        let result = apply(r#"{"text": "hello"}"#, "text", &None, &None).unwrap();
        assert!(result.get("uuid_id").is_none());
    }

    #[test]
    fn preprocess_skips_missing_text_key() {
        assert!(apply(r#"{"text": "kept"}"#, "text", &None, &None).is_some());
        assert!(apply(r#"{"other": "no text field"}"#, "text", &None, &None).is_none());
    }
}
