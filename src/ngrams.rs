use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::parse::Transcription;
use crate::similarity::normalize;

#[derive(Debug, Serialize)]
pub struct NgramResult {
    pub ngram: String,
    pub n: usize,
    pub count: usize,
    pub entry_indices: Vec<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entry_ids: Vec<String>,
}

pub fn extract_ngrams(
    entries: &[Transcription],
    min_n: usize,
    max_n: usize,
    min_count: usize,
    include_ids: bool,
) -> Vec<NgramResult> {
    let normed: Vec<String> = entries.iter().map(|e| normalize(&e.text)).collect();
    let tokenized: Vec<Vec<&str>> = normed
        .iter()
        .map(|n| n.split_whitespace().collect())
        .collect();

    // Collect results grouped by n, also store the word tokens for efficient sub-ngram generation
    let mut results_by_n: HashMap<usize, Vec<(Vec<String>, NgramResult)>> = HashMap::new();

    for n in min_n..=max_n {
        let mut ngram_counts: HashMap<&[&str], Vec<usize>> = HashMap::new();

        for (entry_idx, words) in tokenized.iter().enumerate() {
            if words.len() < n {
                continue;
            }

            let mut seen: HashSet<&[&str]> = HashSet::new();
            for window in words.windows(n) {
                if seen.insert(window) {
                    ngram_counts.entry(window).or_default().push(entry_idx);
                }
            }
        }

        let mut results = Vec::new();
        for (ngram_words, indices) in ngram_counts {
            if indices.len() >= min_count {
                let words: Vec<String> = ngram_words.iter().map(|s| s.to_string()).collect();
                let ngram = words.join(" ");
                let entry_ids = if include_ids {
                    indices.iter().map(|&i| entries[i].id.clone()).collect()
                } else {
                    Vec::new()
                };
                results.push((
                    words,
                    NgramResult {
                        ngram,
                        n,
                        count: indices.len(),
                        entry_indices: indices,
                        entry_ids,
                    },
                ));
            }
        }
        results_by_n.insert(n, results);
    }

    // "Longest phrase wins" deduplication
    // For each longer n-gram, extract all shorter sub-n-grams and mark them
    // as suppressed if they have similar count.
    //
    // Build count lookup per shorter n: ngram_string -> count
    let mut counts_at: HashMap<usize, HashMap<&str, usize>> = HashMap::new();
    for (&n, results) in &results_by_n {
        let mut m = HashMap::new();
        for (_, r) in results {
            m.insert(r.ngram.as_str(), r.count);
        }
        counts_at.insert(n, m);
    }

    let mut suppressed: HashSet<String> = HashSet::new();

    // Process from longest to shortest
    for n in (min_n + 1..=max_n).rev() {
        let Some(longer_results) = results_by_n.get(&n) else {
            continue;
        };

        for (words, result) in longer_results {
            if suppressed.contains(&result.ngram) {
                continue;
            }

            // Generate all sub-n-grams of length sub_n < n
            for sub_n in min_n..n {
                let Some(shorter_counts) = counts_at.get(&sub_n) else {
                    continue;
                };

                for window in words.windows(sub_n) {
                    let sub_ngram = window.join(" ");
                    if let Some(&sub_count) = shorter_counts.get(sub_ngram.as_str())
                        && (sub_count as f64) <= (result.count as f64) * 1.2
                    {
                        suppressed.insert(sub_ngram);
                    }
                }
            }
        }
    }

    let mut kept: Vec<NgramResult> = Vec::new();
    for n in min_n..=max_n {
        if let Some(results) = results_by_n.remove(&n) {
            for (_, r) in results {
                if !suppressed.contains(&r.ngram) {
                    kept.push(r);
                }
            }
        }
    }

    kept.sort_by(|a, b| b.count.cmp(&a.count));
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
    fn finds_repeated_trigram() {
        let entries = vec![
            entry(0, "the quick brown fox"),
            entry(1, "the quick brown dog"),
            entry(2, "the quick brown cat"),
            entry(3, "something else entirely"),
        ];
        let results = extract_ngrams(&entries, 3, 3, 3, false);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].ngram, "the quick brown");
        assert_eq!(results[0].count, 3);
    }

    #[test]
    fn min_count_filters() {
        let entries = vec![
            entry(0, "alpha beta gamma"),
            entry(1, "alpha beta gamma"),
            entry(2, "delta epsilon zeta"),
        ];
        // min_count=3 should find nothing (only 2 occurrences)
        let results = extract_ngrams(&entries, 3, 3, 3, false);
        assert!(results.is_empty());

        // min_count=2 should find it
        let results = extract_ngrams(&entries, 3, 3, 2, false);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn longer_phrase_suppresses_shorter() {
        let entries = vec![
            entry(0, "a b c d"),
            entry(1, "a b c d"),
            entry(2, "a b c d"),
        ];
        // "a b c d" (4-gram) should suppress "a b c" (3-gram) since same count
        let results = extract_ngrams(&entries, 3, 4, 3, false);
        let ngrams: Vec<&str> = results.iter().map(|r| r.ngram.as_str()).collect();
        assert!(ngrams.contains(&"a b c d"));
        assert!(!ngrams.contains(&"a b c"));
    }

    #[test]
    fn empty_entries() {
        let results = extract_ngrams(&[], 3, 5, 2, false);
        assert!(results.is_empty());
    }
}
