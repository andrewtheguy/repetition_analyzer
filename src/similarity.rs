pub fn normalize(text: &str) -> String {
    let lower = text.to_lowercase();
    let cleaned: String = lower
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '\'' { c } else { ' ' })
        .collect();
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Levenshtein distance with early termination when distance exceeds max_dist.
/// Returns None if the distance would exceed max_dist.
pub fn levenshtein_bounded(a: &[u8], b: &[u8], max_dist: usize) -> Option<usize> {
    let m = a.len();
    let n = b.len();

    // Length difference alone exceeds threshold
    if m.abs_diff(n) > max_dist {
        return None;
    }

    if m == 0 {
        return Some(n);
    }
    if n == 0 {
        return Some(m);
    }

    let mut prev = vec![0usize; n + 1];
    let mut curr = vec![0usize; n + 1];

    for (j, val) in prev.iter_mut().enumerate().take(n + 1) {
        *val = j;
    }

    for i in 1..=m {
        curr[0] = i;
        let mut row_min = curr[0];

        // Only compute cells within the band
        let j_start = i.saturating_sub(max_dist);
        let j_end = (i + max_dist + 1).min(n);

        // Fill out-of-band cells with a large value
        if j_start > 0 {
            curr[j_start] = max_dist + 1;
        }

        for j in j_start.max(1)..=j_end.min(n) {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            let val = if j > 0 {
                (prev[j] + 1)
                    .min(curr[j - 1] + 1)
                    .min(prev[j - 1] + cost)
            } else {
                prev[j - 1] + cost
            };
            curr[j] = val;
            row_min = row_min.min(val);
        }

        // Fill remaining out-of-band cells
        for val in &mut curr[(j_end + 1)..=n] {
            *val = max_dist + 1;
        }

        if row_min > max_dist {
            return None;
        }

        std::mem::swap(&mut prev, &mut curr);
    }

    if prev[n] <= max_dist {
        Some(prev[n])
    } else {
        None
    }
}

/// Returns similarity ratio (0.0..=1.0) only if it meets the threshold, else None.
pub fn similarity_above_threshold(a: &str, b: &str, threshold: f64) -> Option<f64> {
    let max_len = a.len().max(b.len());
    if max_len == 0 {
        return Some(1.0);
    }
    let max_dist = ((1.0 - threshold) * max_len as f64).floor() as usize;
    let dist = levenshtein_bounded(a.as_bytes(), b.as_bytes(), max_dist)?;
    let ratio = 1.0 - (dist as f64 / max_len as f64);
    Some(ratio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_basic() {
        assert_eq!(normalize("Hello, World!"), "hello world");
        assert_eq!(normalize("  multiple   spaces  "), "multiple spaces");
        assert_eq!(normalize("it's a test"), "it's a test");
        assert_eq!(normalize("$100 & 50%"), "100 50");
    }

    #[test]
    fn levenshtein_identical() {
        assert_eq!(levenshtein_bounded(b"abc", b"abc", 10), Some(0));
    }

    #[test]
    fn levenshtein_empty() {
        assert_eq!(levenshtein_bounded(b"", b"abc", 10), Some(3));
        assert_eq!(levenshtein_bounded(b"abc", b"", 10), Some(3));
        assert_eq!(levenshtein_bounded(b"", b"", 10), Some(0));
    }

    #[test]
    fn levenshtein_single_edit() {
        assert_eq!(levenshtein_bounded(b"kitten", b"sitten", 10), Some(1));
        assert_eq!(levenshtein_bounded(b"abc", b"abcd", 10), Some(1));
    }

    #[test]
    fn levenshtein_bounded_returns_none() {
        assert_eq!(levenshtein_bounded(b"abc", b"xyz", 1), None);
        assert_eq!(levenshtein_bounded(b"short", b"very long string", 3), None);
    }

    #[test]
    fn similarity_above_threshold_matches() {
        // "abc" vs "abd" — 1 edit out of 3 chars = 0.667 similarity
        assert!(similarity_above_threshold("abc", "abd", 0.5).is_some());
        assert!(similarity_above_threshold("abc", "abd", 0.8).is_none());
    }

    #[test]
    fn similarity_identical() {
        let r = similarity_above_threshold("hello world", "hello world", 0.99);
        assert_eq!(r, Some(1.0));
    }

    #[test]
    fn similarity_empty_strings() {
        assert_eq!(similarity_above_threshold("", "", 0.5), Some(1.0));
    }
}
