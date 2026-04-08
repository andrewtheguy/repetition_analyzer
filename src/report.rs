use serde::Serialize;

use crate::exact::{DuplicateGroup, NearDuplicateCluster};
use crate::near_sequences::NearDuplicateSequence;
use crate::ngrams::NgramResult;
use crate::parse::Transcription;
use crate::sequences::RepeatedSequence;

pub struct ReportData<'a> {
    pub file_path: &'a str,
    pub entries: &'a [Transcription],
    pub duplicates: &'a [DuplicateGroup],
    pub near_dupes: &'a [NearDuplicateCluster],
    pub ngrams: &'a [NgramResult],
    pub sequences: &'a [RepeatedSequence],
    pub near_seqs: &'a [NearDuplicateSequence],
}

#[derive(Serialize)]
struct JsonReport<'a> {
    file_path: &'a str,
    total_entries: usize,
    exact_duplicates: &'a [DuplicateGroup],
    near_duplicates: &'a [NearDuplicateCluster],
    ngrams: &'a [NgramResult],
    repeated_sequences: &'a [RepeatedSequence],
    near_duplicate_sequences: &'a [NearDuplicateSequence],
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{}...", truncated)
    }
}

pub fn print_report(data: &ReportData, top_n: usize) {
    // Header
    println!();
    println!("{}", "=".repeat(70));
    println!("  REPETITION ANALYSIS REPORT");
    println!("  File: {}", data.file_path);
    println!("  Entries: {}", data.entries.len());
    println!("{}", "=".repeat(70));

    // Section 1: Exact Duplicates
    println!();
    println!(
        "--- 1. EXACT DUPLICATES ({} texts appear 2+ times) ---",
        data.duplicates.len()
    );
    println!();

    for (i, group) in data.duplicates.iter().take(top_n).enumerate() {
        println!(
            "  {:>3}. {:>3}x | \"{}\"",
            i + 1,
            group.count,
            truncate(&group.canonical_text, 120)
        );
        let first_id = &group.indices[0].1;
        let last_id = &group.indices.last().unwrap().1;
        println!("        First: id={first_id} | Last: id={last_id}");
        println!();
    }

    // Section 2: Near-Duplicates
    println!(
        "--- 2. NEAR-DUPLICATE CLUSTERS ({} clusters) ---",
        data.near_dupes.len()
    );
    println!();

    for (i, cluster) in data.near_dupes.iter().take(top_n).enumerate() {
        println!(
            "  {:>3}. Cluster ({} variants): \"{}\"",
            i + 1,
            cluster.total_count,
            truncate(&cluster.representative_text, 100)
        );

        // Show up to 5 variant samples
        for (idx, (_entry_idx, _id, text)) in cluster.members.iter().take(5).enumerate() {
            println!("        [{}] \"{}\"", idx + 1, truncate(text, 100));
        }
        if cluster.members.len() > 5 {
            println!(
                "        ... and {} more variants",
                cluster.members.len() - 5
            );
        }
        println!();
    }

    // Section 3: N-grams
    println!("--- 3. MOST REPEATED PHRASES ---");
    println!();

    for (i, ng) in data.ngrams.iter().take(top_n).enumerate() {
        println!(
            "  {:>3}. {:>4}x ({}-gram, {} entries) | \"{}\"",
            i + 1,
            ng.count,
            ng.n,
            ng.entry_indices.len(),
            ng.ngram
        );
    }
    println!();

    // Section 4: Repeated Sequences
    println!(
        "--- 4. REPEATED SEGMENT BLOCKS ({} unique blocks) ---",
        data.sequences.len()
    );
    println!();

    for (i, seq) in data.sequences.iter().take(top_n).enumerate() {
        println!(
            "  {:>3}. {:>3}x | {}-entry block",
            i + 1,
            seq.occurrences.len(),
            seq.length,
        );

        for (j, text) in seq.entry_texts.iter().enumerate() {
            println!("        [{}] \"{}\"", j + 1, truncate(text, 100));
        }

        // Show occurrence start indices
        let indices: Vec<String> = seq
            .occurrences
            .iter()
            .take(10)
            .map(|o| o.start_index.to_string())
            .collect();
        print!("        At index: {}", indices.join(", "));
        if seq.occurrences.len() > 10 {
            print!(" ... +{} more", seq.occurrences.len() - 10);
        }
        println!();
        println!();
    }

    // Section 5: Near-Duplicate Sequences
    println!(
        "--- 5. NEAR-DUPLICATE SEGMENT BLOCKS ({} unique patterns) ---",
        data.near_seqs.len()
    );
    println!();

    for (i, seq) in data.near_seqs.iter().take(top_n).enumerate() {
        println!(
            "  {:>3}. {:>3}x | {}-entry block | avg similarity: {:.1}%",
            i + 1,
            seq.occurrences.len(),
            seq.length,
            seq.avg_similarity * 100.0
        );

        for (occ_idx, occ) in seq.occurrences.iter().take(3).enumerate() {
            println!(
                "        Occurrence {} (index {}):",
                occ_idx + 1,
                occ.start_index
            );
            for (j, text) in occ.entry_texts.iter().enumerate() {
                println!("          [{}] \"{}\"", j + 1, truncate(text, 90));
            }
        }
        if seq.occurrences.len() > 3 {
            println!(
                "        ... +{} more occurrences",
                seq.occurrences.len() - 3
            );
        }
        println!();
    }

    // Section 6: Summary
    println!("--- 6. SUMMARY ---");
    println!();
    println!("  Exact duplicate groups:       {}", data.duplicates.len());
    println!("  Near-duplicate clusters:      {}", data.near_dupes.len());
    println!("  Near-duplicate seq. patterns: {}", data.near_seqs.len());

    if let Some(top_ng) = data.ngrams.first() {
        println!(
            "  Most repeated phrase:         \"{}\" ({}x)",
            truncate(&top_ng.ngram, 60),
            top_ng.count
        );
    }

    if let Some(top_seq) = data.sequences.first() {
        println!(
            "  Most repeated block:          {}-entry block ({}x)",
            top_seq.length,
            top_seq.occurrences.len(),
        );
    }

    println!();
    println!("{}", "=".repeat(70));
}

fn build_json_report<'a>(data: &'a ReportData) -> JsonReport<'a> {
    JsonReport {
        file_path: data.file_path,
        total_entries: data.entries.len(),
        exact_duplicates: data.duplicates,
        near_duplicates: data.near_dupes,
        ngrams: data.ngrams,
        repeated_sequences: data.sequences,
        near_duplicate_sequences: data.near_seqs,
    }
}

pub fn print_json_report(data: &ReportData) {
    let report = build_json_report(data);
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("failed to serialize report")
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn empty_report_data() -> ReportData<'static> {
        ReportData {
            file_path: "test.jsonl",
            entries: &[],
            duplicates: &[],
            near_dupes: &[],
            ngrams: &[],
            sequences: &[],
            near_seqs: &[],
        }
    }

    #[test]
    fn json_report_structure() {
        let data = empty_report_data();
        let report = build_json_report(&data);
        let json: Value =
            serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();
        assert_eq!(json["file_path"], "test.jsonl");
        assert_eq!(json["total_entries"], 0);
    }
}
