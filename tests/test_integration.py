"""Integration test: run full analysis pipeline and verify JSON output structure."""

import json

from repetition_analyzer.analyze import run_analyze


def test_analyze_produces_valid_json(temp_csv, capsys):
    path = temp_csv([
        ["0", "the quick brown fox", "0", "1000", "00:00:00.000", "00:00:01.000"],
        ["1", "the quick brown dog", "1000", "2000", "00:00:01.000", "00:00:02.000"],
        ["2", "the quick brown cat", "2000", "3000", "00:00:02.000", "00:00:03.000"],
        ["3", "something else entirely", "3000", "4000", "00:00:03.000", "00:00:04.000"],
        ["4", "another unique line", "4000", "5000", "00:00:04.000", "00:00:05.000"],
    ])
    run_analyze({"file": path, "format": "json", "min_count": 2, "min_ngram": 3, "max_ngram": 5})
    captured = capsys.readouterr()
    data = json.loads(captured.out)
    assert data["total_entries"] == 5
    assert "exact_duplicates" in data
    assert "near_duplicates" in data
    assert "ngrams" in data
    assert "repeated_sequences" in data
    assert "near_duplicate_sequences" in data


def test_analyze_finds_exact_duplicates(temp_csv, capsys):
    path = temp_csv([
        ["0", "hello world", "0", "1000", "00:00:00.000", "00:00:01.000"],
        ["1", "goodbye world", "1000", "2000", "00:00:01.000", "00:00:02.000"],
        ["2", "hello world", "2000", "3000", "00:00:02.000", "00:00:03.000"],
    ])
    run_analyze({"file": path, "format": "json", "min_count": 2})
    data = json.loads(capsys.readouterr().out)
    assert len(data["exact_duplicates"]) == 1
    assert data["exact_duplicates"][0]["count"] == 2
