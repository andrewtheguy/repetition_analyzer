use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "python", derive(pyo3::FromPyObject))]
pub struct Transcription {
    pub index: usize,
    pub id: String,
    pub text: String,
}

// -- exact.rs types --

#[derive(Debug, Serialize)]
pub struct ExactDuplicateGroup {
    pub canonical_text: String,
    pub count: usize,
    pub indices: Vec<(usize, String)>, // (index, id)
}

#[derive(Debug, Serialize)]
pub struct NearDuplicateCluster {
    pub representative_text: String,
    pub members: Vec<(usize, String, String)>, // (index, id, text)
    pub total_count: usize,
}

// -- ngrams.rs types --

#[derive(Debug, Serialize)]
pub struct NgramResult {
    pub ngram: String,
    pub n: usize,
    pub count: usize,
    pub entry_indices: Vec<(usize, String)>, // (index, id)
}

// -- sequences.rs types --

#[derive(Debug, Serialize)]
pub struct SequenceOccurrence {
    pub start_index: usize,
    pub start_id: String,
}

#[derive(Debug, Serialize)]
pub struct RepeatedSequence {
    pub length: usize,
    pub occurrences: Vec<SequenceOccurrence>,
    pub entry_texts: Vec<String>,
}

// -- near_sequences.rs types --

#[derive(Debug, Serialize)]
pub struct NearSequenceOccurrence {
    pub start_index: usize,
    pub start_id: String,
    pub entry_texts: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct NearDuplicateSequence {
    pub length: usize,
    pub occurrences: Vec<NearSequenceOccurrence>,
    pub representative_texts: Vec<String>,
    pub avg_similarity: f64,
}
