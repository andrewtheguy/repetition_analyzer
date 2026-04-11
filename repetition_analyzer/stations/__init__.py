"""Station-specific segment classifiers."""

from typing import Callable

from . import knx

STATIONS = {
    "knx": knx.classify,
}


def classify(station: str, seg: dict, duration_fn: Callable, text_blob_fn: Callable) -> str:
    classifier = STATIONS.get(station)
    if classifier is None:
        return "other"
    return classifier(seg, duration_fn, text_blob_fn)
