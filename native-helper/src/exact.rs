use std::collections::HashMap;

use crate::similarity::{normalize, similarity_above_threshold};
use crate::types::{NearDuplicateCluster, Transcription};

pub fn find_near_duplicates(
    entries: &[Transcription],
    threshold: f64,
) -> Vec<NearDuplicateCluster> {
    // Bucket by first 3 words of normalized text
    let mut buckets: HashMap<String, Vec<usize>> = HashMap::new();
    let normed: Vec<String> = entries.iter().map(|e| normalize(&e.text)).collect();

    for (i, norm) in normed.iter().enumerate() {
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

    fn entry(index: usize, text: &str) -> Transcription {
        Transcription {
            index,
            id: index.to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn near_duplicates_clustered() {
        let entries = vec![
            entry(0, "The quick brown fox jumps over the lazy dog"),
            entry(1, "The quick brown fox leaps over the lazy dog"),
            entry(2, "Something completely different and unrelated here"),
        ];
        let clusters = find_near_duplicates(&entries, 0.80);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].total_count, 2);
        assert_eq!(clusters[0].members[0].1, "0"); // id
    }

    #[test]
    fn near_duplicates_low_threshold_no_match() {
        let entries = vec![
            entry(0, "The quick brown fox jumps over the lazy dog"),
            entry(1, "The quick brown fox leaps over the lazy dog"),
        ];
        // Very high threshold should not match
        let clusters = find_near_duplicates(&entries, 0.99);
        assert!(clusters.is_empty());
    }

    #[test]
    fn near_duplicates_different_prefix_no_match() {
        // Different first 3 words means different buckets, so no comparison
        let entries = vec![
            entry(0, "Alpha beta gamma some long text here for similarity"),
            entry(1, "Delta epsilon zeta some long text here for similarity"),
        ];
        let clusters = find_near_duplicates(&entries, 0.50);
        assert!(clusters.is_empty());
    }
}
