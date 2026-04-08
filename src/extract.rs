use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use serde::Deserialize;

use crate::error::AppError;
use crate::stations;

// ── Segment (deserialized from extract-unique JSON output) ───────────────

#[derive(Deserialize)]
pub struct Segment {
    #[serde(rename = "type")]
    pub seg_type: String,
    #[allow(dead_code)]
    start_index: usize,
    #[allow(dead_code)]
    end_index: usize,
    pub entry_count: usize,
    pub texts: Vec<String>,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub start_formatted: Option<String>,
    pub end_formatted: Option<String>,
}

impl Segment {
    pub fn duration_secs(&self) -> f64 {
        let s = self.start_ms.unwrap_or(0);
        let e = self.end_ms.unwrap_or(0);
        (e - s) as f64 / 1000.0
    }

    pub fn text_blob(&self) -> String {
        self.texts.join(" ").to_lowercase()
    }

    fn sanitize_ts(ts: &str) -> &str {
        ts.split('.').next().unwrap_or(ts)
    }

    fn filename(&self) -> String {
        let start = self
            .start_formatted
            .as_deref()
            .map(Self::sanitize_ts)
            .unwrap_or("?")
            .replace(':', "_");
        let end = self
            .end_formatted
            .as_deref()
            .map(Self::sanitize_ts)
            .unwrap_or("?")
            .replace(':', "_");
        let tag = &self.seg_type[..1];
        format!("{start}--{end}_{tag}_{}entries.txt", self.entry_count)
    }

    fn header(&self) -> String {
        let start = self
            .start_formatted
            .as_deref()
            .map(Self::sanitize_ts)
            .unwrap_or("?");
        let end = self
            .end_formatted
            .as_deref()
            .map(Self::sanitize_ts)
            .unwrap_or("?");
        format!("{start} - {end} ({} entries)", self.entry_count)
    }
}

// ── Config ───────────────────────────────────────────────────────────────

pub struct ExtractConfig {
    pub segments: String,
    pub station: Option<stations::Station>,
    pub min_entries: usize,
    pub long_threshold: usize,
    pub outdir: String,
}

// ── File writers ─────────────────────────────────────────────────────────

fn write_consolidated_md(
    path: &Path,
    segments: &[&Segment],
    title: &str,
) -> std::io::Result<()> {
    let mut f = fs::File::create(path)?;
    writeln!(f, "# {title}\n")?;
    for seg in segments {
        writeln!(f, "## {}\n", seg.header())?;
        for text in &seg.texts {
            writeln!(f, "{text}\n")?;
        }
        writeln!(f, "---\n")?;
    }
    Ok(())
}

fn write_individual_files(folder: &Path, segments: &[&Segment]) -> std::io::Result<()> {
    fs::create_dir_all(folder)?;
    for seg in segments {
        let mut f = fs::File::create(folder.join(seg.filename()))?;
        for text in &seg.texts {
            writeln!(f, "{text}")?;
        }
    }
    Ok(())
}

fn output_category(
    outdir: &Path,
    category: &str,
    segments: &[&Segment],
    long_threshold: usize,
) -> crate::error::Result<()> {
    let short: Vec<&Segment> = segments
        .iter()
        .filter(|s| s.entry_count < long_threshold)
        .copied()
        .collect();
    let long: Vec<&Segment> = segments
        .iter()
        .filter(|s| s.entry_count >= long_threshold)
        .copied()
        .collect();

    if !short.is_empty() {
        let md_path = outdir.join(format!("{category}_short.md"));
        write_consolidated_md(&md_path, &short, &format!("{category} (short)"))?;
        eprintln!("  {:>4} short -> {}", short.len(), md_path.display());
    }

    if !long.is_empty() {
        let folder = outdir.join(format!("{category}_long"));
        write_individual_files(&folder, &long)?;
        eprintln!("  {:>4} long  -> {}/", long.len(), folder.display());
    }

    Ok(())
}

// ── Entry point ──────────────────────────────────────────────────────────

pub fn run_extract_segments(config: &ExtractConfig) -> crate::error::Result<()> {
    let data = fs::read_to_string(&config.segments).map_err(|e| AppError::FileOpen {
        path: config.segments.clone(),
        source: e,
    })?;
    let all_segments: Vec<Segment> = serde_json::from_str(&data)?;

    let segments: Vec<&Segment> = all_segments
        .iter()
        .filter(|s| s.entry_count >= config.min_entries)
        .collect();

    if segments.is_empty() {
        eprintln!("No segments match the criteria.");
        return Ok(());
    }

    let outdir = Path::new(&config.outdir);
    fs::create_dir_all(outdir)?;

    let unique: Vec<&Segment> = segments.iter().filter(|s| s.seg_type == "unique").copied().collect();
    let repeated: Vec<&Segment> = segments.iter().filter(|s| s.seg_type == "repeated").copied().collect();

    // Unique segments: output as-is
    if !unique.is_empty() {
        eprintln!("Unique: {} segments", unique.len());
        output_category(outdir, "unique", &unique, config.long_threshold)?;
    }

    // Repeated segments: classify per station if specified
    if !repeated.is_empty() {
        if let Some(station) = &config.station {
            let mut categorized: HashMap<String, Vec<&Segment>> = HashMap::new();
            for seg in &repeated {
                let cat = stations::classify(station, seg);
                categorized.entry(cat).or_default().push(seg);
            }

            let mut cats: Vec<_> = categorized.keys().cloned().collect();
            cats.sort();
            for cat in &cats {
                let cat_segs = &categorized[cat];
                eprintln!("Repeated/{cat}: {} segments", cat_segs.len());
                output_category(outdir, &format!("repeated_{cat}"), cat_segs, config.long_threshold)?;
            }
        } else {
            eprintln!("Repeated: {} segments", repeated.len());
            output_category(outdir, "repeated", &repeated, config.long_threshold)?;
        }
    }

    Ok(())
}
