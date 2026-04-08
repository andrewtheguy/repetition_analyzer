mod enrich;
mod error;
mod exact;
mod near_sequences;
mod ngrams;
mod parse;
mod preprocess;
mod report;
mod sequences;
mod similarity;

use std::path::Path;
use std::time::Instant;

use clap::{Parser, Subcommand, ValueEnum};

struct AnalyzeConfig {
    file: String,
    min_ngram: usize,
    max_ngram: usize,
    similarity_threshold: f64,
    top_n: usize,
    min_count: usize,
    min_seq_len: usize,
    max_seq_len: usize,
    min_seq_occurrences: usize,
    seq_similarity_threshold: f64,
    text_key: String,
    id_key: Option<String>,
    format: Format,
}

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
    /// Analyze a preprocessed JSONL file for repeated text
    Analyze {
        /// Path to the preprocessed JSONL file
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

        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },

    /// Enrich a JSON result file with timestamps from the preprocessed JSONL source
    Enrich {
        /// Path to the preprocessed JSONL source file
        #[arg(long)]
        source: String,

        /// Path to the JSON result file from analyze
        #[arg(long)]
        result: String,

        /// JSON key for start time (milliseconds)
        #[arg(long, default_value = "start_ms")]
        start_key: String,

        /// JSON key for end time (milliseconds)
        #[arg(long, default_value = "end_ms")]
        end_key: String,

        /// JSON key for formatted start time
        #[arg(long, default_value = "start_formatted")]
        start_formatted_key: String,

        /// JSON key for formatted end time
        #[arg(long, default_value = "end_formatted")]
        end_formatted_key: String,

        /// Optional JSON key to use as entry ID (must match what was used for analyze)
        #[arg(long)]
        id_key: Option<String>,
    },

    /// [Experimental] Extract one representative per near-duplicate cluster (last occurrence) with timestamps
    ExtractUnique {
        /// Path to the preprocessed JSONL source file
        #[arg(long)]
        source: String,

        /// Path to the JSON result file from analyze
        #[arg(long)]
        result: String,

        /// JSON key for start time (milliseconds)
        #[arg(long, default_value = "start_ms")]
        start_key: String,

        /// JSON key for end time (milliseconds)
        #[arg(long, default_value = "end_ms")]
        end_key: String,

        /// JSON key for formatted start time
        #[arg(long, default_value = "start_formatted")]
        start_formatted_key: String,

        /// JSON key for formatted end time
        #[arg(long, default_value = "end_formatted")]
        end_formatted_key: String,

        /// Optional JSON key to use as entry ID
        #[arg(long)]
        id_key: Option<String>,
    },

    /// Preprocess a JSONL file: apply filters and optionally insert a UUID column
    Preprocess {
        /// Path to the JSONL file
        file: String,

        /// JSON key to use as text content
        #[arg(long, default_value = "text")]
        text_key: String,

        /// Filter entries by key=value or key:type=value
        #[arg(long)]
        filter: Option<String>,

        /// Insert a UUID v7 into each entry under this key name
        #[arg(long)]
        new_id_key: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
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
            format,
        } => run_analyze(&AnalyzeConfig {
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
            format,
        }),
        Command::Enrich {
            source,
            result,
            start_key,
            end_key,
            start_formatted_key,
            end_formatted_key,
            id_key,
        } => enrich::run_enrich(&enrich::EnrichConfig {
            source,
            result,
            start_key,
            end_key,
            start_formatted_key,
            end_formatted_key,
            id_key,
        }),
        Command::ExtractUnique {
            source,
            result,
            start_key,
            end_key,
            start_formatted_key,
            end_formatted_key,
            id_key,
        } => enrich::run_extract_unique(&enrich::EnrichConfig {
            source,
            result,
            start_key,
            end_key,
            start_formatted_key,
            end_formatted_key,
            id_key,
        }),
        Command::Preprocess {
            file,
            text_key,
            filter,
            new_id_key,
        } => preprocess::run_preprocess(&preprocess::PreprocessConfig {
            file,
            text_key,
            filter,
            new_id_key,
        }),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run_analyze(config: &AnalyzeConfig) -> error::Result<()> {
    let start = Instant::now();

    // Parse
    let t = Instant::now();
    eprintln!("Parsing {}...", config.file);
    let parse_opts = parse::ParseOptions {
        text_key: config.text_key.clone(),
        id_key: config.id_key.clone(),
    };
    let entries = parse::parse_jsonl(Path::new(&config.file), &parse_opts)?;
    let include_ids = config.id_key.is_some();
    eprintln!(
        "Loaded {} entries ({:.2}s)",
        entries.len(),
        t.elapsed().as_secs_f64()
    );

    // Exact duplicates
    let t = Instant::now();
    let duplicates = exact::find_exact_duplicates(&entries, include_ids);
    eprintln!(
        "Found {} duplicate groups ({:.2}s)",
        duplicates.len(),
        t.elapsed().as_secs_f64()
    );

    // Near-duplicates
    let t = Instant::now();
    let near_dupes =
        exact::find_near_duplicates(&entries, config.similarity_threshold, include_ids);
    eprintln!(
        "Found {} near-duplicate clusters ({:.2}s)",
        near_dupes.len(),
        t.elapsed().as_secs_f64()
    );

    // N-grams
    let t = Instant::now();
    let ngram_results = ngrams::extract_ngrams(
        &entries,
        config.min_ngram,
        config.max_ngram,
        config.min_count,
        include_ids,
    );
    eprintln!(
        "Found {} significant n-grams ({:.2}s)",
        ngram_results.len(),
        t.elapsed().as_secs_f64()
    );

    // Repeated sequences
    let t = Instant::now();
    let repeated_seqs = sequences::find_repeated_sequences(
        &entries,
        config.min_seq_len,
        config.max_seq_len,
        config.min_seq_occurrences,
        include_ids,
    );
    eprintln!(
        "Found {} repeated sequence patterns ({:.2}s)",
        repeated_seqs.len(),
        t.elapsed().as_secs_f64()
    );

    // Near-duplicate sequences
    let t = Instant::now();
    let near_seqs = near_sequences::find_near_duplicate_sequences(
        &entries,
        config.min_seq_len,
        config.max_seq_len,
        config.seq_similarity_threshold,
        config.min_seq_occurrences,
        &repeated_seqs,
        include_ids,
    );
    eprintln!(
        "Found {} near-duplicate sequence patterns ({:.2}s)",
        near_seqs.len(),
        t.elapsed().as_secs_f64()
    );

    let elapsed = start.elapsed();
    eprintln!("Analysis complete in {:.2}s", elapsed.as_secs_f64());

    // Print report
    let data = report::ReportData {
        file_path: &config.file,
        entries: &entries,
        id_column: config.id_key.as_deref(),
        duplicates: &duplicates,
        near_dupes: &near_dupes,
        ngrams: &ngram_results,
        sequences: &repeated_seqs,
        near_seqs: &near_seqs,
    };
    match config.format {
        Format::Json => report::print_json_report(&data),
        Format::Human => report::print_report(&data, config.top_n),
    }
    Ok(())
}
