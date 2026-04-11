"""Shared test fixtures."""

import csv
import tempfile

import pytest


@pytest.fixture
def temp_csv():
    """Create a temporary CSV file with the canonical format."""
    def _make(rows: list[list[str]]):
        f = tempfile.NamedTemporaryFile(mode="w", suffix=".csv", delete=False, newline="")
        writer = csv.writer(f)
        writer.writerow(["id", "text", "start_ms", "end_ms", "start_formatted", "end_formatted"])
        for row in rows:
            writer.writerow(row)
        f.flush()
        f.close()
        return f.name
    return _make
