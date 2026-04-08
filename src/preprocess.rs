use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use uuid::Uuid;

use crate::parse::{filter_matches, parse_filter, ParsedFilter};

pub struct PreprocessConfig {
    pub file: String,
    pub text_key: String,
    pub filter: Option<String>,
    pub new_id_key: Option<String>,
}

fn process_entries(
    path: &Path,
    text_key: &str,
    filter: &Option<ParsedFilter>,
    new_id_key: &Option<String>,
) -> Vec<Value> {
    let file = File::open(path).expect("Failed to open JSONL file");
    let reader = BufReader::new(file);
    let mut results = Vec::new();

    for line in reader.lines() {
        let line = line.expect("Failed to read line");
        let mut obj: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Apply filter
        if let Some(f) = filter {
            match obj.get(&f.key) {
                Some(v) => match filter_matches(v, f) {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(e) => {
                        eprintln!("Error: {e}");
                        std::process::exit(1);
                    }
                },
                None => continue,
            }
        }

        // Skip entries missing the text key
        if obj.get(text_key).and_then(|v| v.as_str()).is_none() {
            continue;
        }

        // Insert UUID if requested
        if let Some(key) = new_id_key
            && let Some(map) = obj.as_object_mut()
        {
            map.insert(key.clone(), Value::String(Uuid::now_v7().to_string()));
        }

        results.push(obj);
    }

    results
}

pub fn run_preprocess(config: &PreprocessConfig) {
    let parsed_filter = parse_filter(&config.filter);

    let entries = process_entries(
        Path::new(&config.file),
        &config.text_key,
        &parsed_filter,
        &config.new_id_key,
    );

    for entry in &entries {
        println!(
            "{}",
            serde_json::to_string(entry).expect("failed to serialize entry")
        );
    }

    eprintln!("Wrote {} entries", entries.len());
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
    fn preprocess_filters_entries() {
        let f = write_temp_jsonl(&[
            r#"{"type": "meta", "text": "ignored"}"#,
            r#"{"type": "transcription", "text": "kept"}"#,
            r#"{"type": "transcription", "text": "also kept"}"#,
        ]);
        let filter = parse_filter(&Some("type=transcription".to_string()));
        let results = process_entries(f.path(), "text", &filter, &None);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["text"], "kept");
        assert_eq!(results[1]["text"], "also kept");
    }

    #[test]
    fn preprocess_inserts_uuid() {
        let f = write_temp_jsonl(&[
            r#"{"text": "hello"}"#,
            r#"{"text": "world"}"#,
        ]);
        let new_id_key = Some("uuid_id".to_string());
        let results = process_entries(f.path(), "text", &None, &new_id_key);
        assert_eq!(results.len(), 2);

        let id0 = results[0]["uuid_id"].as_str().unwrap();
        let id1 = results[1]["uuid_id"].as_str().unwrap();
        // Valid UUIDs
        assert!(Uuid::parse_str(id0).is_ok());
        assert!(Uuid::parse_str(id1).is_ok());
        // Unique
        assert_ne!(id0, id1);
    }

    #[test]
    fn preprocess_no_uuid_without_new_id_key() {
        let f = write_temp_jsonl(&[r#"{"text": "hello"}"#]);
        let results = process_entries(f.path(), "text", &None, &None);
        assert_eq!(results.len(), 1);
        assert!(results[0].get("uuid_id").is_none());
    }

    #[test]
    fn preprocess_skips_missing_text_key() {
        let f = write_temp_jsonl(&[
            r#"{"text": "kept"}"#,
            r#"{"other": "no text field"}"#,
        ]);
        let results = process_entries(f.path(), "text", &None, &None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["text"], "kept");
    }
}
