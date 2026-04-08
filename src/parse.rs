use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Deserialize)]
struct RawEntry {
    #[serde(rename = "type")]
    entry_type: String,
    start: Option<f64>,
    start_formatted: Option<String>,
    text: Option<String>,
    end: Option<f64>,
    end_formatted: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Transcription {
    pub index: usize,
    pub start: f64,
    pub start_formatted: String,
    pub text: String,
    pub end: f64,
    pub end_formatted: String,
}

pub fn parse_jsonl(path: &Path) -> Vec<Transcription> {
    let file = File::open(path).expect("Failed to open JSONL file");
    let reader = BufReader::new(file);
    let mut transcriptions = Vec::new();
    let mut idx = 0;

    for line in reader.lines() {
        let line = line.expect("Failed to read line");
        let entry: RawEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => continue,
        };

        if entry.entry_type == "transcription" {
            if let (Some(start), Some(start_fmt), Some(text), Some(end), Some(end_fmt)) = (
                entry.start,
                entry.start_formatted,
                entry.text,
                entry.end,
                entry.end_formatted,
            ) {
                transcriptions.push(Transcription {
                    index: idx,
                    start,
                    start_formatted: start_fmt,
                    text,
                    end,
                    end_formatted: end_fmt,
                });
                idx += 1;
            }
        }
    }

    transcriptions
}
