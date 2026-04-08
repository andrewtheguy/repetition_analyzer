use serde::Serialize;

use crate::exact::{DuplicateGroup, NearDuplicateCluster};
use crate::near_sequences::NearDuplicateSequence;
use crate::ngrams::NgramResult;
use crate::parse::Transcription;
use crate::sequences::RepeatedSequence;

#[derive(Serialize)]
pub struct Report<'a> {
    pub file_path: &'a str,
    pub total_entries: usize,
    pub total_duration_secs: f64,
    pub exact_duplicates: &'a [DuplicateGroup],
    pub near_duplicates: &'a [NearDuplicateCluster],
    pub ngrams: &'a [NgramResult],
    pub repeated_sequences: &'a [RepeatedSequence],
    pub near_duplicate_sequences: &'a [NearDuplicateSequence],
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{}...", truncated)
    }
}

fn compute_total_duration(entries: &[Transcription]) -> f64 {
    if let (Some(first), Some(last)) = (entries.first(), entries.last()) {
        last.end - first.start
    } else {
        0.0
    }
}

fn format_duration(secs: f64) -> String {
    let total = secs as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{}h {:02}m {:02}s", h, m, s)
    } else if m > 0 {
        format!("{}m {:02}s", m, s)
    } else {
        format!("{}s", s)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn print_report(
    file_path: &str,
    entries: &[Transcription],
    duplicates: &[DuplicateGroup],
    near_dupes: &[NearDuplicateCluster],
    ngrams: &[NgramResult],
    sequences: &[RepeatedSequence],
    near_seqs: &[NearDuplicateSequence],
    top_n: usize,
) {
    let total_duration = compute_total_duration(entries);

    // Header
    println!();
    println!("{}", "=".repeat(70));
    println!("  REPETITION ANALYSIS REPORT");
    println!("  File: {}", file_path);
    println!(
        "  Entries: {} transcriptions | Duration: {}",
        entries.len(),
        format_duration(total_duration)
    );
    println!("{}", "=".repeat(70));

    // Section 1: Exact Duplicates
    println!();
    println!(
        "--- 1. EXACT DUPLICATES ({} texts appear 2+ times) ---",
        duplicates.len()
    );
    println!();

    for (i, group) in duplicates.iter().take(top_n).enumerate() {
        let first_ts = &entries[group.indices[0]].start_formatted;
        let last_ts = &entries[*group.indices.last().unwrap()].start_formatted;
        println!(
            "  {:>3}. {:>3}x | \"{}\"",
            i + 1,
            group.count,
            truncate(&group.canonical_text, 120)
        );
        println!("        First: {} | Last: {}", first_ts, last_ts);
        println!();
    }

    // Section 2: Near-Duplicates
    println!(
        "--- 2. NEAR-DUPLICATE CLUSTERS ({} clusters) ---",
        near_dupes.len()
    );
    println!();

    for (i, cluster) in near_dupes.iter().take(top_n).enumerate() {
        println!(
            "  {:>3}. Cluster ({} variants): \"{}\"",
            i + 1,
            cluster.total_count,
            truncate(&cluster.representative_text, 100)
        );

        // Show up to 5 variant samples
        for (idx, (_entry_idx, text)) in cluster.members.iter().take(5).enumerate() {
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

    for (i, ng) in ngrams.iter().take(top_n).enumerate() {
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
        sequences.len()
    );
    println!();

    for (i, seq) in sequences.iter().take(top_n).enumerate() {
        println!(
            "  {:>3}. {:>3}x | {}-entry block (~{})",
            i + 1,
            seq.occurrences.len(),
            seq.length,
            format_duration(seq.duration_secs)
        );

        for (j, text) in seq.entry_texts.iter().enumerate() {
            println!("        [{}] \"{}\"", j + 1, truncate(text, 100));
        }

        // Show occurrence timestamps
        let timestamps: Vec<String> = seq
            .occurrences
            .iter()
            .take(10)
            .map(|o| o.start_time.clone())
            .collect();
        print!("        At: {}", timestamps.join(", "));
        if seq.occurrences.len() > 10 {
            print!(" ... +{} more", seq.occurrences.len() - 10);
        }
        println!();
        println!();
    }

    // Section 5: Near-Duplicate Sequences
    println!(
        "--- 5. NEAR-DUPLICATE SEGMENT BLOCKS ({} unique patterns) ---",
        near_seqs.len()
    );
    println!();

    for (i, seq) in near_seqs.iter().take(top_n).enumerate() {
        println!(
            "  {:>3}. {:>3}x | {}-entry block (~{}) | avg similarity: {:.1}%",
            i + 1,
            seq.occurrences.len(),
            seq.length,
            format_duration(seq.duration_secs),
            seq.avg_similarity * 100.0
        );

        for (occ_idx, occ) in seq.occurrences.iter().take(3).enumerate() {
            println!(
                "        Occurrence {} (at {}):",
                occ_idx + 1,
                occ.start_time
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
    println!("  Exact duplicate groups:       {}", duplicates.len());
    println!("  Near-duplicate clusters:      {}", near_dupes.len());
    println!("  Near-duplicate seq. patterns: {}", near_seqs.len());

    if let Some(top_ng) = ngrams.first() {
        println!(
            "  Most repeated phrase:     \"{}\" ({}x)",
            truncate(&top_ng.ngram, 60),
            top_ng.count
        );
    }

    if let Some(top_seq) = sequences.first() {
        println!(
            "  Most repeated block:      {}-entry block ({}x, ~{} each)",
            top_seq.length,
            top_seq.occurrences.len(),
            format_duration(top_seq.duration_secs)
        );
    }

    // Estimate repetition time from exact duplicates
    let mut dup_time = 0.0;
    for group in duplicates {
        // Each occurrence beyond the first is a repetition
        let extra = group.count - 1;
        for &idx in group.indices.iter().skip(1).take(extra) {
            dup_time += entries[idx].end - entries[idx].start;
        }
    }

    if total_duration > 0.0 {
        let pct = (dup_time / total_duration) * 100.0;
        println!(
            "  Est. exact-duplicate time: {} of {} ({:.1}%)",
            format_duration(dup_time),
            format_duration(total_duration),
            pct
        );
    }

    println!();
    println!("{}", "=".repeat(70));
}

pub fn print_json_report(
    file_path: &str,
    entries: &[Transcription],
    duplicates: &[DuplicateGroup],
    near_dupes: &[NearDuplicateCluster],
    ngrams: &[NgramResult],
    sequences: &[RepeatedSequence],
    near_seqs: &[NearDuplicateSequence],
) {
    let total_duration = compute_total_duration(entries);

    let report = Report {
        file_path,
        total_entries: entries.len(),
        total_duration_secs: total_duration,
        exact_duplicates: duplicates,
        near_duplicates: near_dupes,
        ngrams,
        repeated_sequences: sequences,
        near_duplicate_sequences: near_seqs,
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("failed to serialize report")
    );
}
