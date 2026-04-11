"""Tests for CSV parsing."""

import pytest

from repetition_analyzer.parse import parse_csv


def test_parse_canonical_format(temp_csv):
    path = temp_csv([
        ["a1", "hello world", "0", "2500", "00:00:00.000", "00:00:02.500"],
        ["a2", "goodbye", "2500", "5000", "00:00:02.500", "00:00:05.000"],
    ])
    entries = parse_csv(path)
    assert len(entries) == 2
    assert entries[0].id == "a1"
    assert entries[0].text == "hello world"
    assert entries[1].id == "a2"
    assert entries[1].text == "goodbye"
    assert entries[0].index == 0
    assert entries[1].index == 1


def test_parse_text_with_commas(temp_csv):
    path = temp_csv([
        ["a1", "hello, world", "0", "100", "00:00:00.000", "00:00:00.100"],
    ])
    entries = parse_csv(path)
    assert entries[0].text == "hello, world"


def test_parse_duplicate_id_raises(temp_csv):
    path = temp_csv([
        ["same-id", "text a", "0", "100", "00:00:00.000", "00:00:00.100"],
        ["same-id", "text b", "100", "200", "00:00:00.100", "00:00:00.200"],
    ])
    with pytest.raises(ValueError, match="duplicate id"):
        parse_csv(path)


def test_parse_index_matches_line_number(temp_csv):
    path = temp_csv([
        ["1", "a", "", "", "", ""],
        ["2", "b", "", "", "", ""],
        ["3", "c", "", "", "", ""],
    ])
    entries = parse_csv(path)
    assert entries[0].index == 0
    assert entries[1].index == 1
    assert entries[2].index == 2
