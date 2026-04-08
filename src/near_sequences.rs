use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::parse::Transcription;
use crate::sequences::RepeatedSequence;
use crate::similarity::{normalize, similarity_above_threshold};

#[derive(Debug, Serialize)]
pub struct NearSequenceOccurrence {
    pub start_index: usize,
    pub entry_texts: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct NearDuplicateSequence {
    pub length: usize,
    pub occurrences: Vec<NearSequenceOccurrence>,
    pub representative_texts: Vec<String>,
    pub avg_similarity: f64,
}

pub fn find_near_duplicate_sequences(
    entries: &[Transcription],
    min_len: usize,
    max_len: usize,
    threshold: f64,
    min_occurrences: usize,
    exact_sequences: &[RepeatedSequence],
) -> Vec<NearDuplicateSequence> {
    let normed: Vec<String> = entries.iter().map(|e| normalize(&e.text)).collect();

    // Build set of (start_index, length) from exact sequences for filtering
    let mut exact_set: HashSet<(usize, usize)> = HashSet::new();
    for seq in exact_sequences {
        for occ in &seq.occurrences {
            exact_set.insert((occ.start_index, seq.length));
        }
    }

    let mut all_results: Vec<NearDuplicateSequence> = Vec::new();

    for seq_len in (min_len..=max_len).rev() {
        if entries.len() < seq_len {
            continue;
        }

        // Bucket windows by first 3 words of first entry
        let mut buckets: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, norm) in normed.iter().enumerate().take(entries.len() - seq_len + 1) {
            let prefix: String = norm
                .split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ");
            buckets.entry(prefix).or_default().push(i);
        }

        let mut assigned: HashSet<usize> = HashSet::new();

        for bucket_positions in buckets.values() {
            if bucket_positions.len() < min_occurrences {
                continue;
            }

            for &i in bucket_positions {
                if assigned.contains(&i) {
                    continue;
                }

                let mut cluster: Vec<usize> = vec![i];
                let mut cluster_total_sim: f64 = 0.0;
                let mut cluster_comparisons: usize = 0;
                assigned.insert(i);

                for &j in bucket_positions {
                    if assigned.contains(&j) || i == j {
                        continue;
                    }

                    // Length filter on first entry
                    let len_i = normed[i].len();
                    let len_j = normed[j].len();
                    if len_i == 0 || len_j == 0 {
                        continue;
                    }
                    if (len_i.min(len_j) as f64 / len_i.max(len_j) as f64) < 0.7 {
                        continue;
                    }

                    // Entry-by-entry similarity check
                    let mut all_similar = true;
                    let mut pair_sim_total = 0.0;
                    for offset in 0..seq_len {
                        match similarity_above_threshold(
                            &normed[i + offset],
                            &normed[j + offset],
                            threshold,
                        ) {
                            Some(sim) => pair_sim_total += sim,
                            None => {
                                all_similar = false;
                                break;
                            }
                        }
                    }

                    if all_similar {
                        cluster_total_sim += pair_sim_total / seq_len as f64;
                        cluster_comparisons += 1;
                        cluster.push(j);
                        assigned.insert(j);
                    }
                }

                // Un-assign undersized clusters so members can join other clusters
                if cluster.len() < min_occurrences {
                    for &pos in &cluster {
                        assigned.remove(&pos);
                    }
                    continue;
                }

                // Skip if all occurrences already covered by exact sequences
                if cluster
                    .iter()
                    .all(|&pos| exact_set.contains(&(pos, seq_len)))
                {
                    continue;
                }

                let representative_texts: Vec<String> = (0..seq_len)
                    .map(|offset| entries[cluster[0] + offset].text.clone())
                    .collect();

                let occurrences: Vec<NearSequenceOccurrence> = cluster
                    .iter()
                    .map(|&start_idx| NearSequenceOccurrence {
                        start_index: start_idx,
                        entry_texts: (0..seq_len)
                            .map(|offset| entries[start_idx + offset].text.clone())
                            .collect(),
                    })
                    .collect();

                let avg_sim = if cluster_comparisons > 0 {
                    cluster_total_sim / cluster_comparisons as f64
                } else {
                    1.0
                };

                all_results.push(NearDuplicateSequence {
                    length: seq_len,
                    occurrences,
                    representative_texts,
                    avg_similarity: avg_sim,
                });
            }
        }
    }

    // Deduplicate: suppress shorter sequences dominated by longer ones
    all_results.sort_by(|a, b| {
        b.length
            .cmp(&a.length)
            .then_with(|| b.occurrences.len().cmp(&a.occurrences.len()))
    });

    let mut kept: Vec<NearDuplicateSequence> = Vec::new();
    for seq in all_results {
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
    fn finds_near_duplicate_pair() {
        // Two 2-entry blocks with slight text differences
        let entries = vec![
            entry(0, "The quick brown fox jumps over the lazy dog"),
            entry(1, "And then nothing else happened after that event"),
            entry(2, "filler text here"),
            entry(3, "The quick brown fox leaps over the lazy dog"),
            entry(4, "And then nothing else happened after that time"),
        ];
        let exact_seqs = vec![]; // no exact matches
        let seqs = find_near_duplicate_sequences(&entries, 2, 2, 0.80, 2, &exact_seqs);
        assert_eq!(seqs.len(), 1);
        assert_eq!(seqs[0].length, 2);
        assert_eq!(seqs[0].occurrences.len(), 2);
    }

    #[test]
    fn skips_exact_duplicates() {
        // If exact sequences already cover these positions, near-dup should skip
        let entries = vec![
            entry(0, "identical line one"),
            entry(1, "identical line two"),
            entry(2, "filler"),
            entry(3, "identical line one"),
            entry(4, "identical line two"),
        ];
        let exact_seqs = vec![RepeatedSequence {
            length: 2,
            occurrences: vec![
                crate::sequences::SequenceOccurrence { start_index: 0 },
                crate::sequences::SequenceOccurrence { start_index: 3 },
            ],
            entry_texts: vec![
                "identical line one".to_string(),
                "identical line two".to_string(),
            ],
        }];
        let seqs = find_near_duplicate_sequences(&entries, 2, 2, 0.80, 2, &exact_seqs);
        assert!(seqs.is_empty());
    }

    #[test]
    fn no_match_below_threshold() {
        let entries = vec![
            entry(0, "The quick brown fox jumps over the lazy dog"),
            entry(1, "Some other sentence entirely different from anything"),
            entry(2, "filler"),
            entry(3, "The quick brown fox does something completely new today"),
            entry(4, "A totally unrelated sentence about weather and rain"),
        ];
        let seqs = find_near_duplicate_sequences(&entries, 2, 2, 0.95, 2, &[]);
        assert!(seqs.is_empty());
    }
}
