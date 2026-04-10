use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::parse::Transcription;
use crate::similarity::normalize;

#[derive(Debug, Serialize)]
pub struct NgramResult {
    pub ngram: String,
    pub n: usize,
    pub count: usize,
    pub entry_indices: Vec<(usize, String)>, // (index, id)
}

pub fn extract_ngrams(
    entries: &[Transcription],
    min_n: usize,
    max_n: usize,
    min_count: usize,
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
                let count = indices.len();
                let entry_indices = indices
                    .into_iter()
                    .map(|i| (i, entries[i].id.clone()))
                    .collect();
                results.push((
                    words,
                    NgramResult {
                        ngram,
                        n,
                        count,
                        entry_indices,
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

    // Same-length overlap dedup: when two n-grams of the same size overlap
    // by n-1 words (one is a one-word shift of the other) and have similar
    // counts, suppress the lower-count one (or the lexicographically later
    // one if counts are equal).
    for n in min_n..=max_n {
        let Some(results) = results_by_n.get(&n) else {
            continue;
        };
        // Build a lookup from first (n-1) words to the full n-gram
        let mut by_prefix: HashMap<Vec<&str>, Vec<usize>> = HashMap::new();
        for (i, (words, _)) in results.iter().enumerate() {
            let prefix: Vec<&str> = words[..words.len() - 1].iter().map(|s| s.as_str()).collect();
            by_prefix.entry(prefix).or_default().push(i);
        }
        // For each n-gram, check if its suffix matches another n-gram's prefix
        for (i, (words, _)) in results.iter().enumerate() {
            if suppressed.contains(&results[i].1.ngram) {
                continue;
            }
            let suffix: Vec<&str> = words[1..].iter().map(|s| s.as_str()).collect();
            if let Some(matches) = by_prefix.get(&suffix) {
                for &j in matches {
                    if i == j || suppressed.contains(&results[j].1.ngram) {
                        continue;
                    }
                    let ci = results[i].1.count;
                    let cj = results[j].1.count;
                    let (lo, hi) = if ci <= cj { (ci, cj) } else { (cj, ci) };
                    if (lo as f64) >= (hi as f64) * 0.8 {
                        // Similar counts — suppress the lower-count one,
                        // or the later one lexicographically if equal
                        if ci < cj || (ci == cj && results[i].1.ngram > results[j].1.ngram) {
                            suppressed.insert(results[i].1.ngram.clone());
                        } else {
                            suppressed.insert(results[j].1.ngram.clone());
                        }
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
        let results = extract_ngrams(&entries, 3, 3, 3);
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
        let results = extract_ngrams(&entries, 3, 3, 3);
        assert!(results.is_empty());

        // min_count=2 should find it
        let results = extract_ngrams(&entries, 3, 3, 2);
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
        let results = extract_ngrams(&entries, 3, 4, 3);
        let ngrams: Vec<&str> = results.iter().map(|r| r.ngram.as_str()).collect();
        assert!(ngrams.contains(&"a b c d"));
        assert!(!ngrams.contains(&"a b c"));
    }

    #[test]
    fn shifted_same_length_ngrams_consolidated() {
        // "a b c d" and "b c d e" overlap by 3 words — one should be suppressed
        let entries = vec![
            entry(0, "a b c d e"),
            entry(1, "a b c d e"),
            entry(2, "a b c d e"),
        ];
        let results = extract_ngrams(&entries, 4, 4, 3);
        // Should keep only one of "a b c d" or "b c d e", not both
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn shifted_ngrams_different_counts_not_consolidated() {
        // "a b c" appears in 3 entries, "b c d" appears in 5 — counts differ
        // too much (3 < 5*0.8=4), so both should be kept
        let entries = vec![
            entry(0, "a b c d"),
            entry(1, "a b c d"),
            entry(2, "a b c d"),
            entry(3, "x b c d"),
            entry(4, "y b c d"),
        ];
        let results = extract_ngrams(&entries, 3, 3, 3);
        let ngrams: Vec<&str> = results.iter().map(|r| r.ngram.as_str()).collect();
        assert!(ngrams.contains(&"a b c"));
        assert!(ngrams.contains(&"b c d"));
    }

    #[test]
    fn shifted_chain_consolidates_adjacent_pairs() {
        // "a b c d", "b c d e", "c d e f" form a chain — adjacent pairs
        // get consolidated but non-adjacent survivors may remain
        let entries = vec![
            entry(0, "a b c d e f"),
            entry(1, "a b c d e f"),
            entry(2, "a b c d e f"),
        ];
        let results = extract_ngrams(&entries, 4, 4, 3);
        // At least one pair should be consolidated (3 → ≤2)
        assert!(results.len() <= 2);
    }

    #[test]
    fn shifted_ngrams_non_overlapping_both_kept() {
        // "a b c" and "d e f" don't overlap — both kept
        let entries = vec![
            entry(0, "a b c x d e f"),
            entry(1, "a b c y d e f"),
            entry(2, "a b c z d e f"),
        ];
        let results = extract_ngrams(&entries, 3, 3, 3);
        let ngrams: Vec<&str> = results.iter().map(|r| r.ngram.as_str()).collect();
        assert!(ngrams.contains(&"a b c"));
        assert!(ngrams.contains(&"d e f"));
    }

    #[test]
    fn shifted_consolidation_keeps_higher_count() {
        // "a b c" appears 5 times, "b c d" appears 4 times (4/5=0.8, within threshold)
        // Should suppress the lower-count one
        let entries = vec![
            entry(0, "a b c d"),
            entry(1, "a b c d"),
            entry(2, "a b c d"),
            entry(3, "a b c d"),
            entry(4, "a b c x"),
        ];
        let results = extract_ngrams(&entries, 3, 3, 4);
        let ngrams: Vec<&str> = results.iter().map(|r| r.ngram.as_str()).collect();
        assert!(ngrams.contains(&"a b c"));
        assert!(!ngrams.contains(&"b c d"));
    }

    #[test]
    fn empty_entries() {
        let results = extract_ngrams(&[], 3, 5, 2);
        assert!(results.is_empty());
    }

    #[test]
    fn includes_entry_ids() {
        let entries = vec![
            Transcription { index: 0, id: "aaa".to_string(), text: "the quick brown fox".to_string() },
            Transcription { index: 1, id: "bbb".to_string(), text: "the quick brown dog".to_string() },
            Transcription { index: 2, id: "ccc".to_string(), text: "something else entirely".to_string() },
        ];
        let results = extract_ngrams(&entries, 3, 3, 2);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].ngram, "the quick brown");
        let ids: Vec<&str> = results[0].entry_indices.iter().map(|(_, id)| id.as_str()).collect();
        assert_eq!(ids, vec!["aaa", "bbb"]);
    }
}
