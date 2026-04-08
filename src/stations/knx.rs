//! KNX 97.1 FM Los Angeles — segment classifier (work in progress).

use std::sync::LazyLock;

use regex::Regex;

use crate::extract::Segment;

static URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\w+\.(com|org|net|gov|io)\b").unwrap());

static PHONE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d{3}[-.\s]?\d{3}[-.\s]?\d{4}\b").unwrap());

static TOLLFREE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(800|888|877|866|855)\b").unwrap());

static REPORTER_SIGNOFF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"i'm \w+ \w+.*?(k.?n?x|cbs|reporting)").unwrap());

static REPORTER_BYLINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"i'm \w+ \w+[,.]?\s*\w*\s*(news|fm)\b").unwrap());

fn phrase_count(blob: &str, phrases: &[&str]) -> usize {
    phrases.iter().filter(|p| blob.contains(**p)).count()
}

pub fn classify(seg: &Segment) -> String {
    let blob = seg.text_blob();
    let dur = seg.duration_secs();
    let count = seg.entry_count;

    // --- Station bumpers / IDs (short branding) ---
    const BUMPER: &[&str] = &[
        "southern california's news",
        "knx fm",
        "knx news",
        "knx hero of the week",
        "dtla law group",
    ];
    if dur < 25.0 && BUMPER.iter().any(|p| blob.contains(p)) {
        return "station_bumper".into();
    }

    // --- Station promos (longer branding segments) ---
    const PROMO: &[&str] = &[
        "you need to get out more",
        "spotlight on money",
        "grub with greg",
        "taco tuesday",
    ];
    if PROMO.iter().any(|p| blob.contains(p)) {
        return "station_promo".into();
    }

    // --- Weather forecasts ---
    const WEATHER_SIG: &[&str] = &["forecast", "meteorologist", "temperatures"];
    const WEATHER_REG: &[&str] = &["coast", "valleys", "inland empire", "mountains", "high desert"];
    if WEATHER_SIG.iter().any(|s| blob.contains(s))
        && WEATHER_REG.iter().any(|r| blob.contains(r))
    {
        return "weather".into();
    }

    // --- Market updates ---
    const MARKET: &[&str] = &["the dow", "nasdaq", "s&p", "closing bell", "money desk"];
    if phrase_count(&blob, MARKET) >= 2 {
        return "market_update".into();
    }

    // --- Podcast / show promos ---
    const PODCAST: &[&str] = &[
        "podcast",
        "wherever you get your podcasts",
        "follow and listen",
        "on demand",
        "listen on the free",
        "audible original",
    ];
    if PODCAST.iter().any(|p| blob.contains(p)) {
        return "podcast_promo".into();
    }

    // --- Ads ---
    if URL_RE.is_match(&blob) || PHONE_RE.is_match(&blob) || TOLLFREE_RE.is_match(&blob) {
        return "ad".into();
    }
    const AD: &[&str] = &[
        "call now",
        "visit us",
        "limited time",
        "free estimate",
        "free consultation",
        "free quote",
        "absolutely free",
        "no money down",
        "0% apr",
        "apr",
        "cents a day",
        "a month",
        "insurance",
        "credit union",
        "loan",
        "your home",
        "your car",
        "save money",
        "save you money",
        "special offer",
        "promo code",
        "discount",
        "schedule your",
        "book your",
        "workers' comp",
        "injury",
        "attorney",
        "termite",
        "pest",
        "timeshare",
        "exit kit",
        "download our app",
        "great clips",
        "great haircut",
        "see you in the aisles",
    ];
    if phrase_count(&blob, AD) >= 2 {
        return "ad".into();
    }

    // --- Re-aired news packages ---
    if REPORTER_SIGNOFF_RE.is_match(&blob) || REPORTER_BYLINE_RE.is_match(&blob) {
        return "news_package".into();
    }
    if count >= 3 && blob.contains("report") && dur > 20.0 {
        return "news_package".into();
    }

    // --- Expert interviews / analysis segments ---
    const INTERVIEW: &[&str] = &[
        "analyst",
        "professor",
        "expert",
        "your thoughts",
        "do you think",
        "we talked about",
        "we appreciate",
        "climate scientist",
    ];
    if phrase_count(&blob, INTERVIEW) >= 2 {
        return "interview".into();
    }

    "other".into()
}
