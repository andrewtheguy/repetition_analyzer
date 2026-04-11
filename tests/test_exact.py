"""Tests for exact duplicate detection."""

from repetition_analyzer._native import find_all_duplicates
from repetition_analyzer.parse import Entry


def _entry(index, text):
    return Entry(index=index, id=str(index), text=text)


def test_exact_duplicates_found():
    entries = [_entry(0, "Hello world"), _entry(1, "Something else"), _entry(2, "Hello world"), _entry(3, "Hello world")]
    groups, _ = find_all_duplicates(entries, 0.85)
    assert len(groups) == 1
    assert groups[0]["count"] == 3
    indices = [i for i, _ in groups[0]["indices"]]
    assert indices == [0, 2, 3]


def test_exact_duplicates_case_insensitive():
    entries = [_entry(0, "Hello World"), _entry(1, "hello world")]
    groups, _ = find_all_duplicates(entries, 0.85)
    assert len(groups) == 1
    assert groups[0]["count"] == 2


def test_no_duplicates():
    entries = [_entry(0, "alpha"), _entry(1, "beta"), _entry(2, "gamma")]
    groups, _ = find_all_duplicates(entries, 0.85)
    assert len(groups) == 0


def test_near_duplicates_exclude_exact():
    # Entries 0-2 are exact duplicates; entry 3 is a near-duplicate variant.
    # Without exclusion, all four would form a near-duplicate cluster.
    # The fix ensures exact entries are excluded from near-duplicate detection.
    entries = [
        _entry(0, "粵語新聞報道時間"),
        _entry(1, "粵語新聞報道時間"),
        _entry(2, "粵語新聞報道時間"),
        _entry(3, "粵語新聞報導時間到"),
    ]
    exact, near = find_all_duplicates(entries, 0.70)
    assert len(exact) == 1
    assert exact[0]["count"] == 3
    exact_indices = {i for i, _ in exact[0]["indices"]}
    assert exact_indices == {0, 1, 2}
    # Entry 3 alone cannot form a cluster (needs >= 2 members)
    assert len(near) == 0


def test_exact_duplicates_with_ids():
    entries = [
        Entry(index=0, id="aaa", text="Hello world"),
        Entry(index=1, id="bbb", text="other"),
        Entry(index=2, id="ccc", text="Hello world"),
    ]
    groups, _ = find_all_duplicates(entries, 0.85)
    assert len(groups) == 1
    ids = [id_ for _, id_ in groups[0]["indices"]]
    assert ids == ["aaa", "ccc"]
