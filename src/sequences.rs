use std::collections::HashMap;

use serde::Serialize;

use crate::parse::Transcription;
use crate::similarity::normalize;

#[derive(Debug, Serialize)]
pub struct SequenceOccurrence {
    pub start_index: usize,
    pub start_time: String,
}

#[derive(Debug, Serialize)]
pub struct RepeatedSequence {
    pub length: usize,
    pub occurrences: Vec<SequenceOccurrence>,
    pub entry_texts: Vec<String>,
    pub duration_secs: f64,
}

fn fingerprint(text: &str) -> String {
    let norm = normalize(text);
    norm.chars().take(60).collect()
}

pub fn find_repeated_sequences(
    entries: &[Transcription],
    min_len: usize,
    max_len: usize,
    min_occurrences: usize,
) -> Vec<RepeatedSequence> {
    let fingerprints: Vec<String> = entries.iter().map(|e| fingerprint(&e.text)).collect();
    let mut all_sequences: Vec<RepeatedSequence> = Vec::new();

    for seq_len in (min_len..=max_len).rev() {
        if entries.len() < seq_len {
            continue;
        }

        // Build fingerprint windows
        let mut window_map: HashMap<String, Vec<usize>> = HashMap::new();

        for i in 0..=(entries.len() - seq_len) {
            let key: String = fingerprints[i..i + seq_len].join("|");
            window_map.entry(key).or_default().push(i);
        }

        for (_, positions) in window_map {
            if positions.len() < min_occurrences {
                continue;
            }

            // Filter out overlapping occurrences
            let mut filtered = vec![positions[0]];
            for &pos in &positions[1..] {
                if pos >= filtered.last().unwrap() + seq_len {
                    filtered.push(pos);
                }
            }

            if filtered.len() < min_occurrences {
                continue;
            }

            let entry_texts: Vec<String> = (0..seq_len)
                .map(|offset| entries[filtered[0] + offset].text.clone())
                .collect();

            let occurrences: Vec<SequenceOccurrence> = filtered
                .iter()
                .map(|&start_idx| SequenceOccurrence {
                    start_index: start_idx,
                    start_time: entries[start_idx].start_formatted.clone(),
                })
                .collect();

            let duration = entries[filtered[0] + seq_len - 1].end - entries[filtered[0]].start;

            all_sequences.push(RepeatedSequence {
                length: seq_len,
                occurrences,
                entry_texts,
                duration_secs: duration,
            });
        }
    }

    // Deduplicate: suppress shorter sequences that are sub-sequences of longer ones
    // with similar or equal occurrence count
    all_sequences.sort_by(|a, b| {
        b.length
            .cmp(&a.length)
            .then_with(|| b.occurrences.len().cmp(&a.occurrences.len()))
    });

    let mut kept: Vec<RepeatedSequence> = Vec::new();

    for seq in all_sequences {
        let dominated = kept.iter().any(|k| {
            k.length > seq.length
                && k.occurrences.len() >= seq.occurrences.len()
                && seq.occurrences.iter().all(|so| {
                    k.occurrences.iter().any(|ko| {
                        so.start_index >= ko.start_index
                            && so.start_index + seq.length <= ko.start_index + k.length
                    })
                })
        });

        if !dominated {
            kept.push(seq);
        }
    }

    kept.sort_by(|a, b| b.occurrences.len().cmp(&a.occurrences.len()));
    kept
}
