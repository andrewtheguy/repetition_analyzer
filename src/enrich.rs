use serde_json::{Map, Value};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::parse::{filter_matches, parse_filter, ParsedFilter};

pub struct EnrichConfig {
    pub source: String,
    pub result: String,
    pub start_key: String,
    pub end_key: String,
    pub start_formatted_key: String,
    pub end_formatted_key: String,
    pub text_key: String,
    pub filter: Option<String>,
}

struct TimestampInfo {
    start: Option<f64>,
    end: Option<f64>,
    start_formatted: Option<String>,
    end_formatted: Option<String>,
}

fn build_timestamp_lookup(config: &EnrichConfig, filter: &Option<ParsedFilter>) -> Vec<TimestampInfo> {
    let file = File::open(Path::new(&config.source)).expect("Failed to open source JSONL file");
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for line in reader.lines() {
        let line = line.expect("Failed to read line");
        let obj: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Apply filter
        if let Some(f) = filter {
            match obj.get(&f.key) {
                Some(v) if filter_matches(v, f) => {}
                _ => continue,
            }
        }

        // Must have text key to be a valid entry
        if obj.get(&config.text_key).and_then(|v| v.as_str()).is_none() {
            continue;
        }

        entries.push(TimestampInfo {
            start: obj.get(&config.start_key).and_then(|v| v.as_f64()),
            end: obj.get(&config.end_key).and_then(|v| v.as_f64()),
            start_formatted: obj
                .get(&config.start_formatted_key)
                .and_then(|v| v.as_str())
                .map(String::from),
            end_formatted: obj
                .get(&config.end_formatted_key)
                .and_then(|v| v.as_str())
                .map(String::from),
        });
    }

    entries
}

fn enrich_value(value: &mut Value, lookup: &[TimestampInfo]) {
    match value {
        Value::Object(map) => {
            // If this object has a start_index, inject timestamp fields
            if let Some(idx) = map.get("start_index").and_then(|v| v.as_u64())
                && let Some(info) = lookup.get(idx as usize)
            {
                if let Some(s) = info.start {
                    map.insert("start".to_string(), Value::from(s));
                }
                if let Some(e) = info.end {
                    map.insert("end".to_string(), Value::from(e));
                }
                if let Some(sf) = &info.start_formatted {
                    map.insert("start_formatted".to_string(), Value::from(sf.clone()));
                }
                if let Some(ef) = &info.end_formatted {
                    map.insert("end_formatted".to_string(), Value::from(ef.clone()));
                }
            }

            // If this object has an "indices" array (exact_duplicates), add timestamps
            if let Some(Value::Array(indices)) = map.get("indices") {
                let ts_array: Vec<Value> = indices
                    .iter()
                    .filter_map(|v| v.as_u64())
                    .map(|idx| {
                        let mut ts = Map::new();
                        ts.insert("index".to_string(), Value::from(idx));
                        if let Some(info) = lookup.get(idx as usize) {
                            if let Some(s) = info.start {
                                ts.insert("start".to_string(), Value::from(s));
                            }
                            if let Some(e) = info.end {
                                ts.insert("end".to_string(), Value::from(e));
                            }
                            if let Some(sf) = &info.start_formatted {
                                ts.insert("start_formatted".to_string(), Value::from(sf.clone()));
                            }
                            if let Some(ef) = &info.end_formatted {
                                ts.insert("end_formatted".to_string(), Value::from(ef.clone()));
                            }
                        }
                        Value::Object(ts)
                    })
                    .collect();
                map.insert("index_timestamps".to_string(), Value::Array(ts_array));
            }

            // Compute duration for objects that have both start_index and a length
            // (repeated_sequences, near_duplicate_sequences with occurrences)
            // handled via the occurrences enrichment above

            // Recurse into all values
            for v in map.values_mut() {
                enrich_value(v, lookup);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                enrich_value(v, lookup);
            }
        }
        _ => {}
    }
}

pub fn run_enrich(config: &EnrichConfig) {
    let parsed_filter = parse_filter(&config.filter);

    let lookup = build_timestamp_lookup(config, &parsed_filter);

    eprintln!(
        "Loaded {} entries from source for timestamp lookup",
        lookup.len()
    );

    let result_file = File::open(&config.result).expect("Failed to open result JSON file");
    let mut result_json: Value =
        serde_json::from_reader(BufReader::new(result_file)).expect("Failed to parse result JSON");

    // Inject total_duration_secs at top level
    if let Value::Object(ref mut map) = result_json
        && let (Some(first), Some(last)) = (lookup.first(), lookup.last())
        && let (Some(s), Some(e)) = (first.start, last.end)
    {
        map.insert("total_duration_secs".to_string(), Value::from(e - s));
    }

    enrich_value(&mut result_json, &lookup);

    println!(
        "{}",
        serde_json::to_string_pretty(&result_json).expect("failed to serialize enriched result")
    );
}
