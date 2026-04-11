use std::collections::HashMap;

use crate::similarity::{normalize, similarity_above_threshold};
use crate::types::{ExactDuplicateGroup, NearDuplicateCluster, Transcription};

pub fn find_exact_duplicates(entries: &[Transcription]) -> Vec<ExactDuplicateGroup> {
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, entry) in entries.iter().enumerate() {
        let norm = normalize(&entry.text);
        groups.entry(norm).or_default().push(i);
    }

    let mut result: Vec<ExactDuplicateGroup> = groups
        .into_values()
        .filter_map(|indices| {
            // Collapse consecutive indices into single occurrences.
            // Adjacent entries with the same text are often STT hallucinations,
            // not meaningful repetitions.
            let mut deduped: Vec<usize> = Vec::new();
            for &i in &indices {
                if deduped.last().is_none_or(|&prev| entries[i].index != entries[prev].index + 1) {
                    deduped.push(i);
                }
            }
            if deduped.len() < 2 {
                return None;
            }
            let canonical_text = entries[deduped[0]].text.clone();
            let count = deduped.len();
            let index_pairs = deduped
                .iter()
                .map(|&i| (entries[i].index, entries[i].id.clone()))
                .collect();
            Some(ExactDuplicateGroup {
                canonical_text,
                count,
                indices: index_pairs,
            })
        })
        .collect();

    result.sort_by(|a, b| b.count.cmp(&a.count));
    result
}

pub fn find_near_duplicates(
    entries: &[Transcription],
    threshold: f64,
    exact_groups: &[ExactDuplicateGroup],
) -> Vec<NearDuplicateCluster> {
    // Build set of indices already covered by exact duplicate groups
    let mut exact_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for group in exact_groups {
        for &(index, _) in &group.indices {
            exact_indices.insert(index);
        }
    }

    // Bucket by first 3 words of normalized text
    let mut buckets: HashMap<String, Vec<usize>> = HashMap::new();
    let normed: Vec<String> = entries.iter().map(|e| normalize(&e.text)).collect();

    for (i, norm) in normed.iter().enumerate() {
        if exact_indices.contains(&entries[i].index) {
            continue;
        }
        // Skip consecutive duplicates (same normalized text as previous entry)
        if i > 0 && normed[i] == normed[i - 1] {
            continue;
        }
        let prefix: String = norm
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join(" ");
        buckets.entry(prefix).or_default().push(i);
    }

    // Track which entries have been assigned to a cluster
    let mut assigned = vec![false; entries.len()];
    let mut clusters: Vec<NearDuplicateCluster> = Vec::new();

    // Process buckets with multiple entries
    for bucket_indices in buckets.values() {
        if bucket_indices.len() < 2 {
            continue;
        }

        // Within each bucket, cluster by similarity
        for &i in bucket_indices {
            if assigned[i] {
                continue;
            }

            let mut cluster_members = vec![(
                entries[i].index,
                entries[i].id.clone(),
                entries[i].text.clone(),
            )];
            assigned[i] = true;

            for &j in bucket_indices {
                if assigned[j] || i == j {
                    continue;
                }

                // Length filter: skip if lengths differ by more than 30%
                let len_i = normed[i].len();
                let len_j = normed[j].len();
                if len_i == 0 || len_j == 0 {
                    continue;
                }
                let ratio = len_i.min(len_j) as f64 / len_i.max(len_j) as f64;
                if ratio < 0.7 {
                    continue;
                }

                if similarity_above_threshold(&normed[i], &normed[j], threshold).is_some() {
                    cluster_members.push((
                        entries[j].index,
                        entries[j].id.clone(),
                        entries[j].text.clone(),
                    ));
                    assigned[j] = true;
                }
            }

            if cluster_members.len() >= 2 {
                let representative = cluster_members[0].2.clone();
                let total = cluster_members.len();
                clusters.push(NearDuplicateCluster {
                    representative_text: representative,
                    members: cluster_members,
                    total_count: total,
                });
            }
        }
    }

    clusters.sort_by(|a, b| b.total_count.cmp(&a.total_count));
    clusters
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: usize, id: &str, text: &str) -> Transcription {
        Transcription {
            index,
            id: id.to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn exact_duplicates_group_by_normalized_text_and_sort_by_count() {
        let entries = vec![
            entry(10, "alpha", "Hello, World!"),
            entry(20, "beta", "HELLO WORLD"),
            entry(30, "gamma", "hello world??"),
            entry(40, "delta", "A unique entry"),
            entry(50, "epsilon", "Second Group"),
            entry(60, "zeta", "second-group"),
        ];

        assert_eq!(normalize(&entries[0].text), normalize(&entries[1].text));
        assert_eq!(normalize(&entries[1].text), normalize(&entries[2].text));
        assert_eq!(normalize(&entries[4].text), normalize(&entries[5].text));
        assert_ne!(normalize(&entries[0].text), normalize(&entries[3].text));

        let groups: Vec<ExactDuplicateGroup> = find_exact_duplicates(&entries);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].count, 3);
        assert_eq!(groups[1].count, 2);
        assert!(groups[0].count > groups[1].count);

        assert_eq!(groups[0].canonical_text, entries[0].text);
        assert_eq!(
            groups[0].indices,
            vec![
                (entries[0].index, entries[0].id.clone()),
                (entries[1].index, entries[1].id.clone()),
                (entries[2].index, entries[2].id.clone()),
            ]
        );

        assert_eq!(groups[1].canonical_text, entries[4].text);
        assert_eq!(
            groups[1].indices,
            vec![
                (entries[4].index, entries[4].id.clone()),
                (entries[5].index, entries[5].id.clone()),
            ]
        );
    }

    #[test]
    fn exact_duplicates_exclude_singletons() {
        let entries = vec![
            entry(100, "one", "Only once"),
            entry(200, "two", "Still unique"),
            entry(300, "three", "Another unique line"),
        ];

        assert_ne!(normalize(&entries[0].text), normalize(&entries[1].text));
        assert_ne!(normalize(&entries[1].text), normalize(&entries[2].text));

        let groups = find_exact_duplicates(&entries);
        assert!(groups.is_empty());
    }

    #[test]
    fn exact_duplicates_collapse_consecutive() {
        // Consecutive entries with the same text should collapse to one occurrence
        let entries = vec![
            entry(0, "a", "Same text here"),
            entry(1, "b", "Same text here"),
            entry(2, "c", "Different text"),
        ];
        let groups = find_exact_duplicates(&entries);
        assert!(groups.is_empty(), "consecutive-only duplicates should not form a group");
    }

    #[test]
    fn exact_duplicates_consecutive_plus_distant() {
        // Consecutive pair at 0-1, plus a distant occurrence at 10 → 2 logical occurrences
        let entries = vec![
            entry(0, "a", "Repeated line"),
            entry(1, "b", "Repeated line"),
            entry(5, "c", "Something else"),
            entry(10, "d", "Repeated line"),
        ];
        let groups = find_exact_duplicates(&entries);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].count, 2);
        // First occurrence is index 0 (representative of 0-1 run), second is index 10
        assert_eq!(groups[0].indices[0].0, 0);
        assert_eq!(groups[0].indices[1].0, 10);
    }

    #[test]
    fn near_duplicates_clustered() {
        let entries = vec![
            entry(0, "0", "The quick brown fox jumps over the lazy dog"),
            entry(1, "1", "The quick brown fox leaps over the lazy dog"),
            entry(2, "2", "Something completely different and unrelated here"),
        ];
        let clusters = find_near_duplicates(&entries, 0.80, &[]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].total_count, 2);
        assert_eq!(clusters[0].members[0].1, "0"); // id
    }

    #[test]
    fn near_duplicates_low_threshold_no_match() {
        let entries = vec![
            entry(0, "0", "The quick brown fox jumps over the lazy dog"),
            entry(1, "1", "The quick brown fox leaps over the lazy dog"),
        ];
        // Very high threshold should not match
        let clusters = find_near_duplicates(&entries, 0.99, &[]);
        assert!(clusters.is_empty());
    }

    #[test]
    fn near_duplicates_different_prefix_no_match() {
        // Different first 3 words means different buckets, so no comparison
        let entries = vec![
            entry(0, "0", "Alpha beta gamma some long text here for similarity"),
            entry(1, "1", "Delta epsilon zeta some long text here for similarity"),
        ];
        let clusters = find_near_duplicates(&entries, 0.50, &[]);
        assert!(clusters.is_empty());
    }

    #[test]
    fn near_duplicates_excludes_exact_entries() {
        let entries = vec![
            entry(0, "0", "The quick brown fox jumps over the lazy dog"),
            entry(10, "1", "The quick brown fox jumps over the lazy dog"),
            entry(20, "2", "The quick brown fox leaps over the lazy dog"),
        ];
        // Without exclusion, all three would cluster together
        let exact = find_exact_duplicates(&entries);
        assert_eq!(exact.len(), 1);
        assert_eq!(exact[0].count, 2);

        // Near-duplicates should exclude the exact pair (indices 0,10)
        let clusters = find_near_duplicates(&entries, 0.80, &exact);
        assert!(clusters.is_empty());
    }

    #[test]
    fn near_duplicates_skips_consecutive_duplicates() {
        // Consecutive exact duplicates should not form a near-duplicate cluster
        let entries = vec![
            entry(0, "0", "The quick brown fox jumps over the lazy dog"),
            entry(1, "1", "The quick brown fox jumps over the lazy dog"),
            entry(10, "2", "Something completely different and unrelated here"),
        ];
        let exact = find_exact_duplicates(&entries);
        assert!(exact.is_empty(), "consecutive-only pair should not form exact group");
        let clusters = find_near_duplicates(&entries, 0.80, &exact);
        assert!(clusters.is_empty(), "consecutive duplicate should not form near-duplicate cluster");
    }
}
