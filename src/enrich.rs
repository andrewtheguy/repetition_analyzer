use serde_json::{Map, Value};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::error::AppError;

pub struct EnrichConfig {
    pub source: String,
    pub result: String,
}

struct EntryInfo {
    start_ms: Option<i64>,
    end_ms: Option<i64>,
    start_formatted: Option<String>,
    end_formatted: Option<String>,
    id: Option<String>,
}

/// Build lookup from a preprocessed JSONL file (canonical format).
fn build_entry_lookup(source: &str) -> crate::error::Result<Vec<EntryInfo>> {
    let file = File::open(Path::new(source)).map_err(|e| AppError::FileOpen {
        path: source.to_string(),
        source: e,
    })?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| AppError::LineRead {
            line: line_num + 1,
            source: e,
        })?;
        let obj: Value = serde_json::from_str(&line).map_err(|e| AppError::InvalidJson {
            line: line_num + 1,
            source: e,
        })?;

        entries.push(EntryInfo {
            start_ms: obj.get("start_ms").and_then(|v| v.as_i64()),
            end_ms: obj.get("end_ms").and_then(|v| v.as_i64()),
            start_formatted: obj
                .get("start_formatted")
                .and_then(|v| v.as_str())
                .map(String::from),
            end_formatted: obj
                .get("end_formatted")
                .and_then(|v| v.as_str())
                .map(String::from),
            id: obj
                .get("id")
                .and_then(|v| v.as_str())
                .map(String::from),
        });
    }

    Ok(entries)
}

fn inject_entry_info(ts: &mut Map<String, Value>, info: &EntryInfo) {
    if let Some(s) = info.start_ms {
        ts.insert("start_ms".to_string(), Value::from(s));
    }
    if let Some(e) = info.end_ms {
        ts.insert("end_ms".to_string(), Value::from(e));
    }
    if let Some(sf) = &info.start_formatted {
        ts.insert("start_formatted".to_string(), Value::from(sf.clone()));
    }
    if let Some(ef) = &info.end_formatted {
        ts.insert("end_formatted".to_string(), Value::from(ef.clone()));
    }
    if let Some(id) = &info.id {
        ts.insert("id".to_string(), Value::from(id.clone()));
    }
}

fn enrich_value(value: &mut Value, lookup: &[EntryInfo]) {
    match value {
        Value::Object(map) => {
            // If this object has a start_index, inject timestamp and id fields
            if let Some(idx) = map.get("start_index").and_then(|v| v.as_u64())
                && let Some(info) = lookup.get(idx as usize)
            {
                inject_entry_info(map, info);
            }

            // If this object has an "indices" array (exact_duplicates), add timestamps + ids
            if let Some(Value::Array(indices)) = map.get("indices") {
                let ts_array: Vec<Value> = indices
                    .iter()
                    .filter_map(|v| v.as_u64())
                    .map(|idx| {
                        let mut ts = Map::new();
                        ts.insert("index".to_string(), Value::from(idx));
                        if let Some(info) = lookup.get(idx as usize) {
                            inject_entry_info(&mut ts, info);
                        }
                        Value::Object(ts)
                    })
                    .collect();
                map.insert("index_timestamps".to_string(), Value::Array(ts_array));
            }

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

pub fn run_extract_unique(config: &EnrichConfig) -> crate::error::Result<()> {
    let lookup = build_entry_lookup(&config.source)?;

    let result_file = File::open(&config.result).map_err(|e| AppError::FileOpen {
        path: config.result.clone(),
        source: e,
    })?;
    let result_json: Value = serde_json::from_reader(BufReader::new(result_file))?;

    let clusters = result_json
        .get("near_duplicates")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Generic("result JSON has no near_duplicates array".into()))?;

    let mut output: Vec<Value> = Vec::new();

    for cluster in clusters {
        let empty = Vec::new();
        let members = cluster
            .get("members")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty);
        let total_count = cluster.get("total_count").and_then(|v| v.as_u64()).unwrap_or(0);

        // Find the member with the highest index (last occurrence)
        let last = members
            .iter()
            .filter_map(|m| {
                let arr = m.as_array()?;
                let idx = arr.first()?.as_u64()?;
                let text = arr.get(1)?.as_str()?;
                Some((idx, text))
            })
            .max_by_key(|(idx, _)| *idx);

        if let Some((idx, text)) = last {
            let mut entry = Map::new();
            entry.insert("text".to_string(), Value::from(text));
            entry.insert("count".to_string(), Value::from(total_count));
            entry.insert("last_index".to_string(), Value::from(idx));
            if let Some(info) = lookup.get(idx as usize) {
                inject_entry_info(&mut entry, info);
            }
            output.push(Value::Object(entry));
        }
    }

    output.sort_by_key(|v| v.get("last_index").and_then(|i| i.as_u64()).unwrap_or(0));

    let json = serde_json::to_string_pretty(&output)?;
    println!("{json}");
    Ok(())
}

pub fn run_enrich(config: &EnrichConfig) -> crate::error::Result<()> {
    let lookup = build_entry_lookup(&config.source)?;

    eprintln!(
        "Loaded {} entries from source for enrichment lookup",
        lookup.len()
    );

    let result_file = File::open(&config.result).map_err(|e| AppError::FileOpen {
        path: config.result.clone(),
        source: e,
    })?;
    let mut result_json: Value =
        serde_json::from_reader(BufReader::new(result_file))?;

    // Inject total_duration_secs at top level
    if let Value::Object(ref mut map) = result_json
        && let (Some(first), Some(last)) = (lookup.first(), lookup.last())
        && let (Some(s), Some(e)) = (first.start_ms, last.end_ms)
    {
        map.insert(
            "total_duration_secs".to_string(),
            Value::from((e - s) as f64 / 1000.0),
        );
    }

    enrich_value(&mut result_json, &lookup);

    let output = serde_json::to_string_pretty(&result_json)?;
    println!("{output}");
    Ok(())
}
