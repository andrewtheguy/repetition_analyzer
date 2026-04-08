use std::collections::HashMap;

use serde::Serialize;

use crate::parse::Transcription;
use crate::similarity::normalize;

#[derive(Debug, Serialize)]
pub struct SequenceOccurrence {
    pub start_index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RepeatedSequence {
    pub length: usize,
    pub occurrences: Vec<SequenceOccurrence>,
    pub entry_texts: Vec<String>,
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
    include_ids: bool,
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
                    start_id: if include_ids {
                        Some(entries[start_idx].id.clone())
                    } else {
                        None
                    },
                })
                .collect();

            all_sequences.push(RepeatedSequence {
                length: seq_len,
                occurrences,
                entry_texts,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::Transcription;

    fn entry(index: usize, text: &str) -> Transcription {
        Transcription {
            index,
            id: index.to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn finds_repeated_pair() {
        // Pattern "A B" appears at index 0-1 and 3-4
        let entries = vec![
            entry(0, "first line"),
            entry(1, "second line"),
            entry(2, "unrelated stuff"),
            entry(3, "first line"),
            entry(4, "second line"),
        ];
        let seqs = find_repeated_sequences(&entries, 2, 2, 2, false);
        assert_eq!(seqs.len(), 1);
        assert_eq!(seqs[0].length, 2);
        assert_eq!(seqs[0].occurrences.len(), 2);
        assert_eq!(seqs[0].occurrences[0].start_index, 0);
        assert_eq!(seqs[0].occurrences[1].start_index, 3);
    }

    #[test]
    fn no_repeat_below_min_occurrences() {
        let entries = vec![entry(0, "alpha"), entry(1, "beta"), entry(2, "gamma")];
        let seqs = find_repeated_sequences(&entries, 2, 3, 2, false);
        assert!(seqs.is_empty());
    }

    #[test]
    fn overlapping_occurrences_filtered() {
        // "A B" at 0-1, 1-2 would overlap — only non-overlapping kept
        let entries = vec![
            entry(0, "same line"),
            entry(1, "same line"),
            entry(2, "same line"),
        ];
        let seqs = find_repeated_sequences(&entries, 2, 2, 2, false);
        // Entries 0,1 form one block and 2 can't form another (only 1 entry left)
        // But individual entries repeat, so length-1 wouldn't apply (min_len=2)
        // The 2-entry window "same line|same line" appears at 0 and 1, but they overlap
        assert!(seqs.is_empty());
    }

    #[test]
    fn longer_sequence_suppresses_shorter() {
        // 3-entry block at 0-2 and 4-6; shorter 2-entry sub-blocks should be suppressed
        let entries = vec![
            entry(0, "line A"),
            entry(1, "line B"),
            entry(2, "line C"),
            entry(3, "filler"),
            entry(4, "line A"),
            entry(5, "line B"),
            entry(6, "line C"),
        ];
        let seqs = find_repeated_sequences(&entries, 2, 3, 2, false);
        // Should find the 3-entry block and suppress 2-entry sub-blocks
        assert_eq!(seqs.len(), 1);
        assert_eq!(seqs[0].length, 3);
    }
}
