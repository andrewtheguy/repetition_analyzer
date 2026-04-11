"""Station-specific segment classifiers."""

from typing import Any, Callable

from . import knx

STATIONS: dict[str, Callable[[dict[str, Any], Callable[[dict[str, Any]], float], Callable[[dict[str, Any]], str]], str]] = {
    "knx": knx.classify,
}


def classify(station: str, seg: dict[str, Any], duration_fn: Callable[[dict[str, Any]], float], text_blob_fn: Callable[[dict[str, Any]], str]) -> str:
    classifier = STATIONS.get(station)
    if classifier is None:
        return "other"
    return classifier(seg, duration_fn, text_blob_fn)
