mod exact;
mod ngrams;
mod parse;
mod report;
mod sequences;
mod similarity;

use std::path::Path;
use std::time::Instant;

use clap::{Parser, ValueEnum};

#[derive(Clone, Debug, ValueEnum)]
enum Format {
    Human,
    Json,
}

#[derive(Parser)]
#[command(name = "repetition_analyzer")]
#[command(about = "Analyze repeated text in broadcast transcriptions")]
struct Cli {
    /// Path to the JSONL file
    #[arg(default_value = "/Volumes/dasdata/andrewdata/test/knx.jsonl")]
    file: String,

    /// Minimum n-gram size
    #[arg(long, default_value_t = 3)]
    min_ngram: usize,

    /// Maximum n-gram size
    #[arg(long, default_value_t = 8)]
    max_ngram: usize,

    /// Similarity threshold for near-duplicates (0.0 to 1.0)
    #[arg(long, default_value_t = 0.85)]
    similarity_threshold: f64,

    /// Maximum number of results per section
    #[arg(long, default_value_t = 20)]
    top_n: usize,

    /// Minimum repetition count to report
    #[arg(long, default_value_t = 3)]
    min_count: usize,

    /// Minimum block length for repeated sequences
    #[arg(long, default_value_t = 2)]
    min_seq_len: usize,

    /// Maximum block length for repeated sequences
    #[arg(long, default_value_t = 8)]
    max_seq_len: usize,

    /// Minimum occurrences for repeated sequences
    #[arg(long, default_value_t = 2)]
    min_seq_occurrences: usize,

    /// Output format
    #[arg(long, value_enum, default_value_t = Format::Human)]
    format: Format,
}

fn main() {
    let cli = Cli::parse();
    let start = Instant::now();

    // Parse
    let t = Instant::now();
    eprintln!("Parsing {}...", cli.file);
    let entries = parse::parse_jsonl(Path::new(&cli.file));
    eprintln!(
        "Loaded {} transcription entries ({:.2}s)",
        entries.len(),
        t.elapsed().as_secs_f64()
    );

    // Exact duplicates
    let t = Instant::now();
    let duplicates = exact::find_exact_duplicates(&entries);
    eprintln!(
        "Found {} duplicate groups ({:.2}s)",
        duplicates.len(),
        t.elapsed().as_secs_f64()
    );

    // Near-duplicates
    let t = Instant::now();
    let near_dupes = exact::find_near_duplicates(&entries, cli.similarity_threshold);
    eprintln!(
        "Found {} near-duplicate clusters ({:.2}s)",
        near_dupes.len(),
        t.elapsed().as_secs_f64()
    );

    // N-grams
    let t = Instant::now();
    let ngram_results =
        ngrams::extract_ngrams(&entries, cli.min_ngram, cli.max_ngram, cli.min_count);
    eprintln!(
        "Found {} significant n-grams ({:.2}s)",
        ngram_results.len(),
        t.elapsed().as_secs_f64()
    );

    // Repeated sequences
    let t = Instant::now();
    let repeated_seqs = sequences::find_repeated_sequences(
        &entries,
        cli.min_seq_len,
        cli.max_seq_len,
        cli.min_seq_occurrences,
    );
    eprintln!(
        "Found {} repeated sequence patterns ({:.2}s)",
        repeated_seqs.len(),
        t.elapsed().as_secs_f64()
    );

    let elapsed = start.elapsed();
    eprintln!("Analysis complete in {:.2}s", elapsed.as_secs_f64());

    // Print report
    match cli.format {
        Format::Json => report::print_json_report(
            &cli.file,
            &entries,
            &duplicates,
            &near_dupes,
            &ngram_results,
            &repeated_seqs,
        ),
        Format::Human => report::print_report(
            &cli.file,
            &entries,
            &duplicates,
            &near_dupes,
            &ngram_results,
            &repeated_seqs,
            cli.top_n,
        ),
    }
}
