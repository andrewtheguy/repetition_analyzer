"""Tests for exact duplicate detection (Python implementation)."""

from repetition_analyzer.exact import find_exact_duplicates
from repetition_analyzer.parse import Entry


def _entry(index, text):
    return Entry(index=index, id=str(index), text=text)


def test_exact_duplicates_found():
    entries = [_entry(0, "Hello world"), _entry(1, "Something else"), _entry(2, "Hello world"), _entry(3, "Hello world")]
    groups = find_exact_duplicates(entries)
    assert len(groups) == 1
    assert groups[0]["count"] == 3
    indices = [i for i, _ in groups[0]["indices"]]
    assert indices == [0, 2, 3]


def test_exact_duplicates_case_insensitive():
    entries = [_entry(0, "Hello World"), _entry(1, "hello world")]
    groups = find_exact_duplicates(entries)
    assert len(groups) == 1
    assert groups[0]["count"] == 2


def test_no_duplicates():
    entries = [_entry(0, "alpha"), _entry(1, "beta"), _entry(2, "gamma")]
    groups = find_exact_duplicates(entries)
    assert len(groups) == 0


def test_exact_duplicates_with_ids():
    entries = [
        Entry(index=0, id="aaa", text="Hello world"),
        Entry(index=1, id="bbb", text="other"),
        Entry(index=2, id="ccc", text="Hello world"),
    ]
    groups = find_exact_duplicates(entries)
    assert len(groups) == 1
    ids = [id_ for _, id_ in groups[0]["indices"]]
    assert ids == ["aaa", "ccc"]
