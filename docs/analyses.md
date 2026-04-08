# Analysis Methods

All analyses begin by normalizing text: lowercasing and stripping punctuation (except apostrophes). This means "Hello, World!" and "hello world" are treated as equivalent.

## 1. Exact Duplicates

Groups entries whose normalized text is identical. A group is reported when it contains two or more entries.

Each group records the original text, the number of occurrences, and the indices of every matching entry.

**Tuning:** This analysis has no configurable parameters -- all exact matches are reported.

## 2. Near-Duplicate Clusters

Finds entries that are highly similar but not identical, such as minor wording variations or transcription errors.

**How it works:**

- Entries are bucketed by the first three words of their normalized text so that only plausible matches are compared.
- Within each bucket, pairs are compared using Levenshtein distance, converted to a similarity ratio between 0.0 (completely different) and 1.0 (identical).
- Pairs whose similarity meets the threshold are grouped into clusters.
- A length filter skips pairs where the shorter text is less than 70% of the longer text's length, avoiding expensive comparisons that can't possibly meet the threshold.

**Tuning:**

| Option | Effect |
|---|---|
| `--similarity-threshold` | Lower values find more distant matches; higher values are stricter. The default (0.85) catches minor variations while avoiding false positives. |

## 3. Repeated Phrases (N-grams)

Identifies word sequences (n-grams) that appear across multiple entries.

**How it works:**

- Each entry's normalized text is split into words.
- For each n-gram size from `--min-ngram` to `--max-ngram`, a sliding window extracts all word sequences of that length.
- N-grams that appear in fewer than `--min-count` entries are discarded.
- Deduplication suppresses shorter sub-phrases when a longer phrase accounts for most of their occurrences. For example, if "welcome back to the show" appears 10 times, the sub-phrase "welcome back" won't be separately reported at a similar count.
- Results are sorted by occurrence count (descending).

**Tuning:**

| Option | Effect |
|---|---|
| `--min-ngram` | Shortest phrase length in words. Raising this filters out common short phrases. |
| `--max-ngram` | Longest phrase length in words. |
| `--min-count` | Minimum number of entries a phrase must appear in. Higher values surface only the most frequent phrases. |

## 4. Repeated Sequences (Block Detection)

Detects contiguous multi-entry blocks that repeat elsewhere in the data. Unlike n-gram analysis (which works at the word level within a single entry), this operates at the entry level -- looking for runs of consecutive entries that reappear as a group.

**How it works:**

- Each entry is reduced to a fingerprint (the first 60 characters of its normalized text).
- For each block length from `--max-seq-len` down to `--min-seq-len`, a sliding window of fingerprints identifies blocks that appear at least `--min-seq-occurrences` times.
- Overlapping occurrences are filtered out so each reported occurrence is distinct.
- Shorter blocks that are fully contained within longer blocks (with similar occurrence counts) are suppressed.

**Tuning:**

| Option | Effect |
|---|---|
| `--min-seq-len` | Minimum number of consecutive entries in a block. |
| `--max-seq-len` | Maximum block length. Larger values can find longer repeated segments but take more time. |
| `--min-seq-occurrences` | How many times a block must appear to be reported. |

## 5. Near-Duplicate Sequences (Block-Level Fuzzy Matching)

Detects contiguous multi-entry blocks that repeat with minor text variations. This fills the gap between exact sequence matching (which requires identical fingerprints) and single-entry near-duplicate clustering (which doesn't span entry boundaries). Useful for catching recurring segments like news reports, traffic updates, or ad reads where transcription varies slightly between airings.

**How it works:**

- For each block length from `--max-seq-len` down to `--min-seq-len`, a sliding window generates candidate blocks.
- Candidate blocks are bucketed by the first three words of the first entry (same bucketing strategy as single-entry near-duplicate detection) to avoid exhaustive pairwise comparison.
- Within each bucket, blocks are compared entry-by-entry: every corresponding entry pair must individually meet the similarity threshold. This preserves alignment (entry boundaries are stable) and allows early termination when any entry pair fails.
- Blocks are grouped using greedy star-clustering: a representative is chosen, and all sufficiently similar blocks are added to its cluster.
- Clusters where every occurrence is already covered by an exact repeated sequence are filtered out to avoid redundant reporting.
- Shorter near-duplicate sequences that are fully contained within longer ones (with equal or greater occurrence counts) are suppressed.

**Tuning:**

| Option | Effect |
|---|---|
| `--seq-similarity-threshold` | Per-entry similarity threshold for block matching. Default is 0.80, slightly lower than the single-entry threshold (0.85), because requiring ALL entries to pass is already a strict aggregate constraint. |
| `--min-seq-len` | Minimum entries in a block (shared with exact sequences). |
| `--max-seq-len` | Maximum entries in a block (shared with exact sequences). |
| `--min-seq-occurrences` | Minimum times a block must repeat (shared with exact sequences). |
