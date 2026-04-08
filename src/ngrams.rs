use std::collections::{HashMap, HashSet};

use crate::parse::Transcription;
use crate::similarity::normalize;

#[derive(Debug)]
pub struct NgramResult {
    pub ngram: String,
    pub n: usize,
    pub count: usize,
    pub entry_indices: Vec<usize>,
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
                results.push((
                    words,
                    NgramResult {
                        ngram,
                        n,
                        count: indices.len(),
                        entry_indices: indices,
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
                    if let Some(&sub_count) = shorter_counts.get(sub_ngram.as_str()) {
                        if (sub_count as f64) <= (result.count as f64) * 1.2 {
                            suppressed.insert(sub_ngram);
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
