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
    /// Analyze a preprocessed CSV file for repeated text
    Analyze {
        /// Path to the preprocessed CSV file
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

        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },

    /// Enrich a JSON result file with timestamps from the preprocessed CSV source
    Enrich {
        /// Path to the preprocessed CSV source file
        #[arg(long)]
        source: String,

        /// Path to the JSON result file from analyze
        #[arg(long)]
        result: String,
    },

    /// Segment entries into contiguous unique/repeated ranges based on all repetition analyses
    ExtractUnique {
        /// Path to the preprocessed CSV source file
        #[arg(long)]
        source: String,

        /// Path to the JSON result file from analyze
        #[arg(long)]
        result: String,
    },

    /// Preprocess a JSONL file into CSV: filter, normalize field names, and ensure unique IDs
    Preprocess {
        /// Path to the JSONL file
        file: String,

        /// Input JSON key for text content
        #[arg(long, default_value = "text")]
        text_key: String,

        /// Input JSON key for existing unique ID (omit to auto-generate UUIDv7)
        #[arg(long)]
        id_key: Option<String>,

        /// Input JSON key for start time in milliseconds
        #[arg(long, default_value = "start_ms")]
        start_ms_key: String,

        /// Input JSON key for end time in milliseconds
        #[arg(long, default_value = "end_ms")]
        end_ms_key: String,

        /// Input JSON key for formatted start time (HH:MM:SS.mmm)
        #[arg(long, default_value = "start_formatted")]
        start_formatted_key: String,

        /// Input JSON key for formatted end time (HH:MM:SS.mmm)
        #[arg(long, default_value = "end_formatted")]
        end_formatted_key: String,

        /// Filter entries by key=value or key:type=value
        #[arg(long)]
        filter: Option<String>,
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
            format,
        }),
        Command::Enrich { source, result } => {
            enrich::run_enrich(&enrich::EnrichConfig { source, result })
        }
        Command::ExtractUnique { source, result } => {
            enrich::run_extract_unique(&enrich::EnrichConfig { source, result })
        }
        Command::Preprocess {
            file,
            text_key,
            id_key,
            start_ms_key,
            end_ms_key,
            start_formatted_key,
            end_formatted_key,
            filter,
        } => preprocess::run_preprocess(&preprocess::PreprocessConfig {
            file,
            text_key,
            id_key,
            start_ms_key,
            end_ms_key,
            start_formatted_key,
            end_formatted_key,
            filter,
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
    let entries = parse::parse_csv(Path::new(&config.file))?;
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
    let near_dupes =
        exact::find_near_duplicates(&entries, config.similarity_threshold);
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
