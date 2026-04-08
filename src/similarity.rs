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
