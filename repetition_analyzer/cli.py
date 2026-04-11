"""Command-line interface for repetition_analyzer."""

import argparse
import sys


def main():
    parser = argparse.ArgumentParser(
        prog="repetition-analyzer",
        description="Analyze repeated text in JSONL files",
    )
    subparsers = parser.add_subparsers(dest="command")

    # -- preprocess --
    pp = subparsers.add_parser("preprocess", help="Preprocess a JSONL file into CSV")
    pp.add_argument("file", help="Path to the JSONL file")
    pp.add_argument("--text-key", default="text", help="Input JSON key for text content")
    pp.add_argument("--id-key", default=None, help="Input JSON key for existing unique ID")
    pp.add_argument("--start-ms-key", default="start_ms", help="Input JSON key for start time in ms")
    pp.add_argument("--end-ms-key", default="end_ms", help="Input JSON key for end time in ms")
    pp.add_argument("--start-formatted-key", default="start_formatted", help="Input JSON key for formatted start time")
    pp.add_argument("--end-formatted-key", default="end_formatted", help="Input JSON key for formatted end time")
    pp.add_argument("--filter", default=None, help="Filter entries by key=value or key:type=value")

    # -- analyze --
    an = subparsers.add_parser("analyze", help="Analyze a preprocessed CSV file for repeated text")
    an.add_argument("file", help="Path to the preprocessed CSV file")
    an.add_argument("--min-ngram", type=int, default=3, help="Minimum n-gram size")
    an.add_argument("--max-ngram", type=int, default=8, help="Maximum n-gram size")
    an.add_argument("--similarity-threshold", type=float, default=0.85, help="Similarity threshold for near-duplicates")
    an.add_argument("--top-n", type=int, default=20, help="Maximum results per section")
    an.add_argument("--min-count", type=int, default=3, help="Minimum repetition count")
    an.add_argument("--min-seq-len", type=int, default=2, help="Minimum block length for sequences")
    an.add_argument("--max-seq-len", type=int, default=8, help="Maximum block length for sequences")
    an.add_argument("--min-seq-occurrences", type=int, default=2, help="Minimum occurrences for sequences")
    an.add_argument("--seq-similarity-threshold", type=float, default=0.80, help="Similarity threshold for near-dup sequences")
    an.add_argument("--format", choices=["human", "json"], default="human", help="Output format")

    # -- enrich --
    en = subparsers.add_parser("enrich", help="Enrich a JSON result with timestamps from CSV source")
    en.add_argument("--source", required=True, help="Path to the preprocessed CSV source file")
    en.add_argument("--result", required=True, help="Path to the JSON result file from analyze")

    # -- extract-unique --
    eu = subparsers.add_parser("extract-unique", help="Segment entries into unique/repeated ranges")
    eu.add_argument("--source", required=True, help="Path to the preprocessed CSV source file")
    eu.add_argument("--result", required=True, help="Path to the JSON result file from analyze")

    # -- extract-segments --
    es = subparsers.add_parser("extract-segments", help="Extract segments to files")
    es.add_argument("--segments", required=True, help="Path to the segments JSON from extract-unique")
    es.add_argument("--min-entries", type=int, default=3, help="Minimum entry count to include")
    es.add_argument("--long-threshold", type=int, default=10, help="Segments >= this go to individual files")
    es.add_argument("--outdir", required=True, help="Output directory")

    # -- clip --
    cl = subparsers.add_parser("clip", help="Clip CSV to a time range")
    cl.add_argument("file", help="Path to the preprocessed CSV file")
    cl.add_argument("--after", default=None, help="Drop entries starting after this time (HH:MM:SS.mmm or milliseconds)")
    cl.add_argument("--before", default=None, help="Drop entries starting before this time (HH:MM:SS.mmm or milliseconds)")

    # -- detect-rebroadcast (experimental) --
    dr = subparsers.add_parser("detect-rebroadcast", help="[experimental] Detect rebroadcast blocks in a CSV file")
    dr.add_argument("file", help="Path to the preprocessed CSV file")
    dr.add_argument("--min-block-length", type=int, default=10, help="Minimum matched entries per block")
    dr.add_argument("--similarity-threshold", type=float, default=0.7, help="Minimum similarity for a match")
    dr.add_argument("--min-gap", type=int, default=50, help="Minimum index distance between original and rebroadcast")
    dr.add_argument("--merge-gap", type=int, default=5, help="Merge blocks within this many entries of each other")

    # -- plot --
    pl = subparsers.add_parser("plot", help="Generate HTML visualization from enriched JSON")
    pl.add_argument("file", help="Path to the enriched JSON file")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        sys.exit(1)

    if args.command == "preprocess":
        from .preprocess import run_preprocess
        run_preprocess({
            "file": args.file,
            "text_key": args.text_key,
            "id_key": args.id_key,
            "start_ms_key": args.start_ms_key,
            "end_ms_key": args.end_ms_key,
            "start_formatted_key": args.start_formatted_key,
            "end_formatted_key": args.end_formatted_key,
            "filter": args.filter,
        })
    elif args.command == "analyze":
        from .analyze import run_analyze
        run_analyze({
            "file": args.file,
            "min_ngram": args.min_ngram,
            "max_ngram": args.max_ngram,
            "similarity_threshold": args.similarity_threshold,
            "top_n": args.top_n,
            "min_count": args.min_count,
            "min_seq_len": args.min_seq_len,
            "max_seq_len": args.max_seq_len,
            "min_seq_occurrences": args.min_seq_occurrences,
            "seq_similarity_threshold": args.seq_similarity_threshold,
            "format": args.format,
        })
    elif args.command == "enrich":
        from .enrich import run_enrich
        run_enrich({"source": args.source, "result": args.result})
    elif args.command == "extract-unique":
        from .enrich import run_extract_unique
        run_extract_unique({"source": args.source, "result": args.result})
    elif args.command == "extract-segments":
        from .extract import run_extract_segments
        run_extract_segments({
            "segments": args.segments,
            "min_entries": args.min_entries,
            "long_threshold": args.long_threshold,
            "outdir": args.outdir,
        })
    elif args.command == "clip":
        from .clip import run_clip
        run_clip({
            "file": args.file,
            "after": args.after,
            "before": args.before,
        })
    elif args.command == "detect-rebroadcast":
        from .rebroadcast import run_detect_rebroadcast
        run_detect_rebroadcast({
            "file": args.file,
            "min_block_length": args.min_block_length,
            "similarity_threshold": args.similarity_threshold,
            "min_gap": args.min_gap,
            "merge_gap": args.merge_gap,
        })
    elif args.command == "plot":
        from .plot import run_plot
        run_plot(args.file)
