from typing import TypedDict

from .parse import Entry

IndexId = tuple[int, str]
IndexIdText = tuple[int, str, str]


class ExactDuplicateGroup(TypedDict):
    canonical_text: str
    count: int
    indices: list[IndexId]


class NearDuplicateCluster(TypedDict):
    representative_text: str
    members: list[IndexIdText]
    total_count: int


class NgramResult(TypedDict):
    ngram: str
    n: int
    count: int
    entry_indices: list[IndexId]


class SequenceOccurrence(TypedDict):
    start_index: int
    start_id: str


class RepeatedSequence(TypedDict):
    length: int
    occurrences: list[SequenceOccurrence]
    entry_texts: list[str]


class NearSequenceOccurrence(TypedDict):
    start_index: int
    start_id: str
    entry_texts: list[str]


class NearDuplicateSequence(TypedDict):
    length: int
    occurrences: list[NearSequenceOccurrence]
    representative_texts: list[str]
    avg_similarity: float


def find_exact_duplicates(entries: list[Entry]) -> list[ExactDuplicateGroup]: ...
def find_near_duplicates(entries: list[Entry], threshold: float) -> list[NearDuplicateCluster]: ...
def extract_ngrams(entries: list[Entry], min_n: int, max_n: int, min_count: int) -> list[NgramResult]: ...
def find_all_sequences(
    entries: list[Entry],
    min_len: int,
    max_len: int,
    min_occurrences: int,
    similarity_threshold: float,
) -> tuple[list[RepeatedSequence], list[NearDuplicateSequence]]: ...
