"""KNX 97.1 FM Los Angeles — segment classifier."""

import re


def _text_blob(seg: dict) -> str:
    return " ".join(seg.get("texts", [])).lower()


def _seg_duration_s(seg: dict) -> float:
    s = seg.get("start_ms", 0)
    e = seg.get("end_ms", 0)
    return (e - s) / 1000.0


def classify(seg: dict) -> str:
    blob = _text_blob(seg)
    dur = _seg_duration_s(seg)
    count = seg.get("entry_count", 0)

    # --- Station bumpers / IDs ---
    bumper_phrases = [
        "southern california's news",
        "knx fm",
        "knx news",
        "knx hero of the week",
        "dtla law group",
    ]
    if dur < 25 and any(p in blob for p in bumper_phrases):
        return "station_bumper"

    # --- Station promos (longer branding segments) ---
    promo_phrases = [
        "you need to get out more",
        "spotlight on money",
        "grub with greg",
        "taco tuesday",
    ]
    if any(p in blob for p in promo_phrases):
        return "station_promo"

    # --- Weather forecasts ---
    weather_signals = ["forecast", "meteorologist", "temperatures"]
    weather_regions = ["coast", "valleys", "inland empire", "mountains", "high desert"]
    if any(s in blob for s in weather_signals) and any(r in blob for r in weather_regions):
        return "weather"

    # --- Market updates ---
    market_phrases = ["the dow", "nasdaq", "s&p", "closing bell", "money desk"]
    if sum(1 for p in market_phrases if p in blob) >= 2:
        return "market_update"

    # --- Podcast / show promos ---
    if any(p in blob for p in [
        "podcast", "wherever you get your podcasts",
        "follow and listen", "on demand",
        "listen on the free", "audible original",
    ]):
        return "podcast_promo"

    # --- Ads ---
    # URL patterns
    if re.search(r'\b\w+\.(com|org|net|gov|io)\b', blob):
        return "ad"
    # Phone numbers
    if re.search(r'\b\d{3}[-.\s]?\d{3}[-.\s]?\d{4}\b', blob):
        return "ad"
    if re.search(r'\b(800|888|877|866|855)\b', blob):
        return "ad"
    # Price / offer / product language
    ad_phrases = [
        "call now", "visit us", "limited time",
        "free estimate", "free consultation", "free quote",
        "absolutely free", "no money down",
        "0% apr", "apr",
        "cents a day", "a month",
        "insurance", "credit union", "loan",
        "your home", "your car",
        "save money", "save you money",
        "special offer", "promo code", "discount",
        "schedule your", "book your",
        "workers' comp", "injury", "attorney",
        "termite", "pest",
        "timeshare", "exit kit",
        "download our app",
        "great clips", "great haircut",
        "see you in the aisles",
    ]
    if sum(1 for p in ad_phrases if p in blob) >= 2:
        return "ad"

    # --- Re-aired news packages ---
    # Reporter sign-offs (tolerant of transcription errors: kx, kdx, knx)
    if re.search(r"i'm \w+ \w+.*?(k.?n?x|cbs|reporting)", blob):
        return "news_package"
    if re.search(r"i'm \w+ \w+[,.]?\s*\w*\s*(news|fm)\b", blob):
        return "news_package"
    if count >= 3 and re.search(r"(reports?|reporting)\b", blob) and dur > 20:
        return "news_package"

    # --- Expert interviews / analysis segments ---
    interview_phrases = [
        "analyst", "professor", "expert",
        "your thoughts", "do you think",
        "we talked about", "we appreciate",
        "middle east", "climate scientist",
    ]
    if sum(1 for p in interview_phrases if p in blob) >= 2:
        return "interview"

    return "other"
