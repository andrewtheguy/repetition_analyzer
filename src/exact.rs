use std::collections::HashMap;

use serde::Serialize;

use crate::parse::Transcription;
use crate::similarity::{normalize, similarity_above_threshold};

#[derive(Debug, Serialize)]
pub struct DuplicateGroup {
    pub canonical_text: String,
    pub count: usize,
    pub indices: Vec<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
}

pub fn find_exact_duplicates(
    entries: &[Transcription],
    include_ids: bool,
) -> Vec<DuplicateGroup> {
    let mut map: HashMap<String, Vec<usize>> = HashMap::new();

    for entry in entries {
        let norm = normalize(&entry.text);
        map.entry(norm).or_default().push(entry.index);
    }

    let mut groups: Vec<DuplicateGroup> = map
        .into_iter()
        .filter(|(_, indices)| indices.len() >= 2)
        .map(|(_, indices)| {
            let canonical = entries[indices[0]].text.clone();
            let count = indices.len();
            let ids = if include_ids {
                indices.iter().map(|&i| entries[i].id.clone()).collect()
            } else {
                Vec::new()
            };
            DuplicateGroup {
                canonical_text: canonical,
                count,
                indices,
                ids,
            }
        })
        .collect();

    groups.sort_by(|a, b| b.count.cmp(&a.count));
    groups
}

#[derive(Debug, Serialize)]
pub struct NearDuplicateCluster {
    pub representative_text: String,
    pub members: Vec<(usize, String)>, // (index, text)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub member_ids: Vec<String>,
    pub total_count: usize,
}

pub fn find_near_duplicates(
    entries: &[Transcription],
    threshold: f64,
    include_ids: bool,
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

            let mut cluster_members = vec![(entries[i].index, entries[i].text.clone())];
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
                    cluster_members.push((entries[j].index, entries[j].text.clone()));
                    assigned[j] = true;
                }
            }

            if cluster_members.len() >= 2 {
                let representative = cluster_members[0].1.clone();
                let total = cluster_members.len();
                let member_ids = if include_ids {
                    cluster_members
                        .iter()
                        .map(|(idx, _)| entries[*idx].id.clone())
                        .collect()
                } else {
                    Vec::new()
                };
                clusters.push(NearDuplicateCluster {
                    representative_text: representative,
                    members: cluster_members,
                    member_ids,
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
    use crate::parse::Transcription;

    fn entry(index: usize, text: &str) -> Transcription {
        Transcription {
            index,
            id: index.to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn exact_duplicates_found() {
        let entries = vec![
            entry(0, "Hello world"),
            entry(1, "Something else"),
            entry(2, "Hello world"),
            entry(3, "Hello world"),
        ];
        let groups = find_exact_duplicates(&entries, false);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].count, 3);
        assert_eq!(groups[0].indices, vec![0, 2, 3]);
        assert!(groups[0].ids.is_empty());
    }

    #[test]
    fn exact_duplicates_case_insensitive() {
        let entries = vec![entry(0, "Hello World"), entry(1, "hello world")];
        let groups = find_exact_duplicates(&entries, false);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].count, 2);
    }

    #[test]
    fn no_duplicates() {
        let entries = vec![entry(0, "alpha"), entry(1, "beta"), entry(2, "gamma")];
        let groups = find_exact_duplicates(&entries, false);
        assert!(groups.is_empty());
    }

    #[test]
    fn exact_duplicates_with_ids() {
        let entries = vec![
            Transcription { index: 0, id: "aaa".to_string(), text: "Hello world".to_string() },
            Transcription { index: 1, id: "bbb".to_string(), text: "other".to_string() },
            Transcription { index: 2, id: "ccc".to_string(), text: "Hello world".to_string() },
        ];
        let groups = find_exact_duplicates(&entries, true);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].ids, vec!["aaa", "ccc"]);
    }

    #[test]
    fn near_duplicates_clustered() {
        let entries = vec![
            entry(0, "The quick brown fox jumps over the lazy dog"),
            entry(1, "The quick brown fox leaps over the lazy dog"),
            entry(2, "Something completely different and unrelated here"),
        ];
        let clusters = find_near_duplicates(&entries, 0.80, false);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].total_count, 2);
        assert!(clusters[0].member_ids.is_empty());
    }

    #[test]
    fn near_duplicates_low_threshold_no_match() {
        let entries = vec![
            entry(0, "The quick brown fox jumps over the lazy dog"),
            entry(1, "The quick brown fox leaps over the lazy dog"),
        ];
        // Very high threshold should not match
        let clusters = find_near_duplicates(&entries, 0.99, false);
        assert!(clusters.is_empty());
    }

    #[test]
    fn near_duplicates_different_prefix_no_match() {
        // Different first 3 words means different buckets, so no comparison
        let entries = vec![
            entry(0, "Alpha beta gamma some long text here for similarity"),
            entry(1, "Delta epsilon zeta some long text here for similarity"),
        ];
        let clusters = find_near_duplicates(&entries, 0.50, false);
        assert!(clusters.is_empty());
    }
}
