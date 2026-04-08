use serde::Serialize;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct Transcription {
    pub index: usize,
    pub id: String,
    pub text: String,
}

pub struct ParseOptions {
    pub text_key: String,
    pub id_key: Option<String>,
    pub filter_key: Option<String>,
    pub filter_value: Option<String>,
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
        if let (Some(fk), Some(fv)) = (&opts.filter_key, &opts.filter_value) {
            match obj.get(fk) {
                Some(Value::String(s)) if s == fv => {}
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
            filter_key: None,
            filter_value: None,
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
            filter_key: None,
            filter_value: None,
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
            filter_key: None,
            filter_value: None,
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
            filter_key: Some("type".to_string()),
            filter_value: Some("transcription".to_string()),
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
            filter_key: None,
            filter_value: None,
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
            filter_key: None,
            filter_value: None,
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
            filter_key: None,
            filter_value: None,
        };
        let entries = parse_jsonl(f.path(), &opts);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "valid");
    }
}
