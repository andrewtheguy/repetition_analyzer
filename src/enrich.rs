use std::collections::HashSet;

use serde_json::{Map, Value};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::error::AppError;

pub struct EnrichConfig {
    pub source: String,
    pub result: String,
}

struct EntryInfo {
    text: Option<String>,
    start_ms: Option<i64>,
    end_ms: Option<i64>,
    start_formatted: Option<String>,
    end_formatted: Option<String>,
    id: Option<String>,
}

/// CSV column indices: id,text,start_ms,end_ms,start_formatted,end_formatted
const COL_ID: usize = 0;
const COL_TEXT: usize = 1;
const COL_START_MS: usize = 2;
const COL_END_MS: usize = 3;
const COL_START_FMT: usize = 4;
const COL_END_FMT: usize = 5;

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() { None } else { Some(s.to_string()) }
}

/// Build lookup from a preprocessed CSV file (canonical format).
fn build_entry_lookup(source: &str) -> crate::error::Result<Vec<EntryInfo>> {
    let file = File::open(Path::new(source)).map_err(|e| AppError::FileOpen {
        path: source.to_string(),
        source: e,
    })?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(BufReader::new(file));
    let mut entries = Vec::new();

    for (line_num, result) in rdr.records().enumerate() {
        let record = result.map_err(|e| AppError::Generic(format!("line {}: {e}", line_num + 1)))?;

        entries.push(EntryInfo {
            id: record.get(COL_ID).and_then(non_empty),
            text: record.get(COL_TEXT).and_then(non_empty),
            start_ms: record.get(COL_START_MS).and_then(|s| s.parse().ok()),
            end_ms: record.get(COL_END_MS).and_then(|s| s.parse().ok()),
            start_formatted: record.get(COL_START_FMT).and_then(non_empty),
            end_formatted: record.get(COL_END_FMT).and_then(non_empty),
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
    if let Some(text) = &info.text {
        ts.insert("text".to_string(), Value::from(text.clone()));
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

            // For arrays whose elements start with [index, ...], inject timestamps.
            // Covers: "indices" (exact_duplicates), "members" (near_duplicates),
            // "entry_indices" (ngrams).
            for key in ["indices", "members", "entry_indices"] {
                if let Some(Value::Array(arr)) = map.get(key) {
                    let ts_array: Vec<Value> = arr
                        .iter()
                        .filter_map(|v| {
                            let a = v.as_array()?;
                            let idx = a.first()?.as_u64()?;
                            Some(idx)
                        })
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

/// Collect all entry indices that are covered by any repetition pattern.
fn collect_repeated_indices(result_json: &Value) -> HashSet<usize> {
    let mut repeated = HashSet::new();

    // From exact_duplicates: indices are [[index, id], ...]
    if let Some(Value::Array(groups)) = result_json.get("exact_duplicates") {
        for group in groups {
            if let Some(Value::Array(indices)) = group.get("indices") {
                for entry in indices {
                    if let Some(idx) = entry.as_array().and_then(|a| a.first()?.as_u64()) {
                        repeated.insert(idx as usize);
                    }
                }
            }
        }
    }

    // From near_duplicates: members are [[index, id, text], ...]
    if let Some(Value::Array(clusters)) = result_json.get("near_duplicates") {
        for cluster in clusters {
            if let Some(Value::Array(members)) = cluster.get("members") {
                for member in members {
                    if let Some(idx) = member.as_array().and_then(|a| a.first()?.as_u64()) {
                        repeated.insert(idx as usize);
                    }
                }
            }
        }
    }

    // From repeated_sequences: each occurrence covers [start_index, start_index + length)
    if let Some(Value::Array(seqs)) = result_json.get("repeated_sequences") {
        for seq in seqs {
            let length = seq.get("length").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            if let Some(Value::Array(occurrences)) = seq.get("occurrences") {
                for occ in occurrences {
                    if let Some(start) = occ.get("start_index").and_then(|v| v.as_u64()) {
                        for offset in 0..length {
                            repeated.insert(start as usize + offset);
                        }
                    }
                }
            }
        }
    }

    // From near_duplicate_sequences: same range logic
    if let Some(Value::Array(seqs)) = result_json.get("near_duplicate_sequences") {
        for seq in seqs {
            let length = seq.get("length").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            if let Some(Value::Array(occurrences)) = seq.get("occurrences") {
                for occ in occurrences {
                    if let Some(start) = occ.get("start_index").and_then(|v| v.as_u64()) {
                        for offset in 0..length {
                            repeated.insert(start as usize + offset);
                        }
                    }
                }
            }
        }
    }

    repeated
}

fn build_segment(lookup: &[EntryInfo], start: usize, end: usize, is_repeated: bool) -> Value {
    let mut seg = Map::new();
    seg.insert(
        "type".to_string(),
        Value::from(if is_repeated { "repeated" } else { "unique" }),
    );
    seg.insert("start_index".to_string(), Value::from(start));
    seg.insert("end_index".to_string(), Value::from(end));
    seg.insert("entry_count".to_string(), Value::from(end - start + 1));

    let texts: Vec<Value> = (start..=end)
        .filter_map(|i| lookup.get(i)?.text.as_deref())
        .map(Value::from)
        .collect();
    seg.insert("texts".to_string(), Value::Array(texts));

    // Timestamps: start from first entry, end from last entry
    let first = lookup.get(start);
    let last = lookup.get(end);
    seg.insert(
        "start_ms".to_string(),
        Value::from(first.and_then(|e| e.start_ms).unwrap_or(0)),
    );
    seg.insert(
        "end_ms".to_string(),
        Value::from(last.and_then(|e| e.end_ms).unwrap_or(0)),
    );
    if let Some(sf) = first.and_then(|e| e.start_formatted.as_ref()) {
        seg.insert("start_formatted".to_string(), Value::from(sf.clone()));
    }
    if let Some(ef) = last.and_then(|e| e.end_formatted.as_ref()) {
        seg.insert("end_formatted".to_string(), Value::from(ef.clone()));
    }

    Value::Object(seg)
}

pub fn run_extract_unique(config: &EnrichConfig) -> crate::error::Result<()> {
    let lookup = build_entry_lookup(&config.source)?;
    let total = lookup.len();

    let result_file = File::open(&config.result).map_err(|e| AppError::FileOpen {
        path: config.result.clone(),
        source: e,
    })?;
    let result_json: Value = serde_json::from_reader(BufReader::new(result_file))?;

    let repeated = collect_repeated_indices(&result_json);

    eprintln!(
        "{} / {} entries covered by repetition patterns",
        repeated.len(),
        total
    );

    // Walk entries and group consecutive same-type indices into segments
    let mut segments: Vec<Value> = Vec::new();
    if total > 0 {
        let mut seg_start = 0usize;
        let mut seg_repeated = repeated.contains(&0);

        for i in 1..total {
            let is_rep = repeated.contains(&i);
            if is_rep != seg_repeated {
                segments.push(build_segment(&lookup, seg_start, i - 1, seg_repeated));
                seg_start = i;
                seg_repeated = is_rep;
            }
        }
        segments.push(build_segment(&lookup, seg_start, total - 1, seg_repeated));
    }

    let json = serde_json::to_string_pretty(&segments)?;
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
