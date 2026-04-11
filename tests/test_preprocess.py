"""Tests for preprocessing logic."""

import pytest

from repetition_analyzer.preprocess import (
    formatted_to_ms,
    ms_to_formatted,
    parse_filter,
    process_entry,
)


def _default_config():
    return {
        "file": "",
        "text_key": "text",
        "id_key": None,
        "start_ms_key": "start_ms",
        "end_ms_key": "end_ms",
        "start_formatted_key": "start_formatted",
        "end_formatted_key": "end_formatted",
    }


def _apply(obj, config=None, parsed_filter=None):
    config = config or _default_config()
    return process_entry(obj, config, parsed_filter)


def test_ms_to_formatted():
    assert ms_to_formatted(0) == "00:00:00.000"
    assert ms_to_formatted(7552) == "00:00:07.552"
    assert ms_to_formatted(3661001) == "01:01:01.001"


def test_formatted_to_ms():
    assert formatted_to_ms("00:00:00.000") == 0
    assert formatted_to_ms("00:00:07.552") == 7552
    assert formatted_to_ms("01:01:01.001") == 3661001


def test_filters_entries():
    parsed_filter = parse_filter("type=transcript")
    assert _apply({"type": "meta", "text": "ignored", "start_ms": 0, "end_ms": 100}, parsed_filter=parsed_filter) is None
    result = _apply({"type": "transcript", "text": "kept", "start_ms": 0, "end_ms": 100}, parsed_filter=parsed_filter)
    assert result["text"] == "kept"


def test_generates_uuid_without_id_key():
    r0 = _apply({"text": "hello", "start_ms": 0, "end_ms": 100})
    r1 = _apply({"text": "world", "start_ms": 100, "end_ms": 200})
    assert r0["id"] != r1["id"]
    assert len(r0["id"]) > 0


def test_uses_existing_id_key():
    config = _default_config()
    config["id_key"] = "uid"
    result = _apply({"text": "hello", "uid": "abc-123", "start_ms": 0, "end_ms": 100}, config=config)
    assert result["id"] == "abc-123"


def test_skips_missing_text_key():
    assert _apply({"text": "kept", "start_ms": 0, "end_ms": 100}) is not None
    assert _apply({"other": "no text field", "start_ms": 0, "end_ms": 100}) is None


def test_converts_ms_to_formatted():
    result = _apply({"text": "hi", "start_ms": 7552, "end_ms": 90061001})
    assert result["start_formatted"] == "00:00:07.552"
    assert result["end_formatted"] == "25:01:01.001"


def test_converts_formatted_to_ms():
    result = _apply({"text": "hi", "start_formatted": "01:30:05.250", "end_formatted": "02:00:00.000"})
    assert result["start_ms"] == "5405250"
    assert result["end_ms"] == "7200000"


def test_errors_on_missing_start_timestamp():
    with pytest.raises(ValueError, match="missing start timestamp"):
        _apply({"text": "hi", "end_ms": 100})


def test_errors_on_missing_end_timestamp():
    with pytest.raises(ValueError, match="missing end timestamp"):
        _apply({"text": "hi", "start_ms": 0})


def test_errors_on_invalid_formatted_timestamp():
    with pytest.raises(ValueError, match="invalid start timestamp format"):
        _apply({"text": "hi", "start_formatted": "bad", "end_formatted": "00:00:01.000"})
