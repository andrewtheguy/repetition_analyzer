mod enrich;
mod exact;
mod near_sequences;
mod ngrams;
mod parse;
mod report;
mod sequences;
mod similarity;

use std::path::Path;
use std::time::Instant;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Clone, Debug, ValueEnum)]
enum Format {
    Human,
    Json,
}

#[derive(Parser)]
#[command(name = "repetition_analyzer")]
#[command(about = "Analyze repeated text in JSONL files")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Analyze a JSONL file for repeated text
    Analyze {
        /// Path to the JSONL file
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

        /// Similarity threshold for near-duplicate sequences (0.0 to 1.0)
        #[arg(long, default_value_t = 0.80)]
        seq_similarity_threshold: f64,

        /// JSON key to use as text content
        #[arg(long, default_value = "text")]
        text_key: String,

        /// Optional JSON key to use as entry ID (defaults to file line number)
        #[arg(long)]
        id_key: Option<String>,

        /// Filter entries by key=value (e.g., --filter type=transcription)
        #[arg(long)]
        filter: Option<String>,

        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },

    /// Enrich a JSON result file with timestamps from the original JSONL source
    Enrich {
        /// Path to the original JSONL source file
        #[arg(long)]
        source: String,

        /// Path to the JSON result file from analyze
        #[arg(long)]
        result: String,

        /// JSON key for start time (seconds)
        #[arg(long, default_value = "start")]
        start_key: String,

        /// JSON key for end time (seconds)
        #[arg(long, default_value = "end")]
        end_key: String,

        /// JSON key for formatted start time
        #[arg(long, default_value = "start_formatted")]
        start_formatted_key: String,

        /// JSON key for formatted end time
        #[arg(long, default_value = "end_formatted")]
        end_formatted_key: String,

        /// JSON key to use as text content (for matching entries)
        #[arg(long, default_value = "text")]
        text_key: String,

        /// Filter entries by key=value (must match what was used for analyze)
        #[arg(long)]
        filter: Option<String>,
    },
}

fn parse_filter(filter: &Option<String>) -> (Option<String>, Option<String>) {
    match filter {
        Some(f) => {
            let (k, v) = f
                .split_once('=')
                .expect("--filter must be in key=value format");
            (Some(k.to_string()), Some(v.to_string()))
        }
        None => (None, None),
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Analyze {
            file,
            min_ngram,
            max_ngram,
            similarity_threshold,
            top_n,
            min_count,
            min_seq_len,
            max_seq_len,
            min_seq_occurrences,
            seq_similarity_threshold,
            text_key,
            id_key,
            filter,
            format,
        } => {
            run_analyze(
                &file,
                min_ngram,
                max_ngram,
                similarity_threshold,
                top_n,
                min_count,
                min_seq_len,
                max_seq_len,
                min_seq_occurrences,
                seq_similarity_threshold,
                &text_key,
                &id_key,
                &filter,
                &format,
            );
        }
        Command::Enrich {
            source,
            result,
            start_key,
            end_key,
            start_formatted_key,
            end_formatted_key,
            text_key,
            filter,
        } => {
            enrich::run_enrich(
                &source,
                &result,
                &start_key,
                &end_key,
                &start_formatted_key,
                &end_formatted_key,
                &text_key,
                &filter,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_analyze(
    file: &str,
    min_ngram: usize,
    max_ngram: usize,
    similarity_threshold: f64,
    top_n: usize,
    min_count: usize,
    min_seq_len: usize,
    max_seq_len: usize,
    min_seq_occurrences: usize,
    seq_similarity_threshold: f64,
    text_key: &str,
    id_key: &Option<String>,
    filter: &Option<String>,
    format: &Format,
) {
    let start = Instant::now();

    // Parse
    let t = Instant::now();
    eprintln!("Parsing {}...", file);
    let (filter_key, filter_value) = parse_filter(filter);
    let parse_opts = parse::ParseOptions {
        text_key: text_key.to_string(),
        id_key: id_key.clone(),
        filter_key,
        filter_value,
    };
    let entries = parse::parse_jsonl(Path::new(file), &parse_opts);
    eprintln!(
        "Loaded {} entries ({:.2}s)",
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
    let near_dupes = exact::find_near_duplicates(&entries, similarity_threshold);
    eprintln!(
        "Found {} near-duplicate clusters ({:.2}s)",
        near_dupes.len(),
        t.elapsed().as_secs_f64()
    );

    // N-grams
    let t = Instant::now();
    let ngram_results = ngrams::extract_ngrams(&entries, min_ngram, max_ngram, min_count);
    eprintln!(
        "Found {} significant n-grams ({:.2}s)",
        ngram_results.len(),
        t.elapsed().as_secs_f64()
    );

    // Repeated sequences
    let t = Instant::now();
    let repeated_seqs =
        sequences::find_repeated_sequences(&entries, min_seq_len, max_seq_len, min_seq_occurrences);
    eprintln!(
        "Found {} repeated sequence patterns ({:.2}s)",
        repeated_seqs.len(),
        t.elapsed().as_secs_f64()
    );

    // Near-duplicate sequences
    let t = Instant::now();
    let near_seqs = near_sequences::find_near_duplicate_sequences(
        &entries,
        min_seq_len,
        max_seq_len,
        seq_similarity_threshold,
        min_seq_occurrences,
        &repeated_seqs,
    );
    eprintln!(
        "Found {} near-duplicate sequence patterns ({:.2}s)",
        near_seqs.len(),
        t.elapsed().as_secs_f64()
    );

    let elapsed = start.elapsed();
    eprintln!("Analysis complete in {:.2}s", elapsed.as_secs_f64());

    // Print report
    match format {
        Format::Json => report::print_json_report(
            file,
            &entries,
            &duplicates,
            &near_dupes,
            &ngram_results,
            &repeated_seqs,
            &near_seqs,
        ),
        Format::Human => report::print_report(
            file,
            &entries,
            &duplicates,
            &near_dupes,
            &ngram_results,
            &repeated_seqs,
            &near_seqs,
            top_n,
        ),
    }
}
