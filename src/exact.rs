use std::collections::HashMap;

use serde::Serialize;

use crate::parse::Transcription;
use crate::similarity::{normalize, similarity_above_threshold};

#[derive(Debug, Serialize)]
pub struct DuplicateGroup {
    pub canonical_text: String,
    pub count: usize,
    pub indices: Vec<usize>,
}

pub fn find_exact_duplicates(entries: &[Transcription]) -> Vec<DuplicateGroup> {
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
            DuplicateGroup {
                canonical_text: canonical,
                count,
                indices,
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
    pub total_count: usize,
}

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
