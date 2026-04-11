"""KNX 97.1 FM Los Angeles — segment classifier."""

import re
from typing import Callable

URL_RE = re.compile(r"\b\w+\.(com|org|net|gov|io)\b")
PHONE_RE = re.compile(r"\b\d{3}[-.\s]?\d{3}[-.\s]?\d{4}\b")
TOLLFREE_RE = re.compile(r"\b(800|888|877|866|855)\b")
REPORTER_SIGNOFF_RE = re.compile(r"i'm \w+ \w+.*?(k.?n?x|cbs|reporting)")
REPORTER_BYLINE_RE = re.compile(r"i'm \w+ \w+[,.]?\s*\w*\s*(news|fm)\b")

BUMPER = [
    "southern california's news",
    "knx fm",
    "knx news",
    "knx hero of the week",
    "dtla law group",
]

PROMO = [
    "you need to get out more",
    "spotlight on money",
    "grub with greg",
    "taco tuesday",
]

WEATHER_SIG = ["forecast", "meteorologist", "temperatures"]
WEATHER_REG = ["coast", "valleys", "inland empire", "mountains", "high desert"]

MARKET = ["the dow", "nasdaq", "s&p", "closing bell", "money desk"]

PODCAST = [
    "podcast",
    "wherever you get your podcasts",
    "follow and listen",
    "on demand",
    "listen on the free",
    "audible original",
]

AD = [
    "call now", "visit us", "limited time", "free estimate", "free consultation",
    "free quote", "absolutely free", "no money down", "0% apr", "apr",
    "cents a day", "a month", "insurance", "credit union", "loan",
    "your home", "your car", "save money", "save you money", "special offer",
    "promo code", "discount", "schedule your", "book your", "workers' comp",
    "injury", "attorney", "termite", "pest", "timeshare", "exit kit",
    "download our app", "great clips", "great haircut", "see you in the aisles",
]

INTERVIEW = [
    "analyst", "professor", "expert", "your thoughts", "do you think",
    "we talked about", "we appreciate", "climate scientist",
]


def _phrase_count(blob: str, phrases: list[str]) -> int:
    return sum(1 for p in phrases if p in blob)


def classify(seg: dict, duration_fn: Callable, text_blob_fn: Callable) -> str:
    blob = text_blob_fn(seg)
    dur = duration_fn(seg)
    count = seg.get("entry_count", 0)

    # Station bumpers / IDs
    if dur < 25.0 and any(p in blob for p in BUMPER):
        return "station_bumper"

    # Station promos
    if any(p in blob for p in PROMO):
        return "station_promo"

    # Weather
    if any(s in blob for s in WEATHER_SIG) and any(r in blob for r in WEATHER_REG):
        return "weather"

    # Market updates
    if _phrase_count(blob, MARKET) >= 2:
        return "market_update"

    # Podcast promos
    if any(p in blob for p in PODCAST):
        return "podcast_promo"

    # Ads
    if URL_RE.search(blob) or PHONE_RE.search(blob) or TOLLFREE_RE.search(blob):
        return "ad"
    if _phrase_count(blob, AD) >= 2:
        return "ad"

    # News packages
    if REPORTER_SIGNOFF_RE.search(blob) or REPORTER_BYLINE_RE.search(blob):
        return "news_package"
    if count >= 3 and "report" in blob and dur > 20.0:
        return "news_package"

    # Interviews
    if _phrase_count(blob, INTERVIEW) >= 2:
        return "interview"

    return "other"
