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
