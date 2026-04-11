use pyo3::prelude::*;
use pythonize::pythonize;

use crate::types::{RepeatedSequence, SequenceOccurrence, Transcription};
use crate::{
    extract_ngrams as rust_extract_ngrams,
    find_near_duplicate_sequences as rust_find_near_duplicate_sequences,
    find_near_duplicates as rust_find_near_duplicates,
    find_repeated_sequences as rust_find_repeated_sequences, normalize as rust_normalize,
};

fn to_transcriptions(entries: Vec<(usize, String, String)>) -> Vec<Transcription> {
    entries
        .into_iter()
        .map(|(index, id, text)| Transcription { index, id, text })
        .collect()
}

#[pyfunction(name = "normalize")]
fn normalize_py(text: &str) -> String {
    rust_normalize(text)
}

#[pyfunction(name = "find_near_duplicates")]
fn find_near_duplicates_py<'py>(
    py: Python<'py>,
    entries: Vec<(usize, String, String)>,
    threshold: f64,
) -> PyResult<Bound<'py, PyAny>> {
    let result = rust_find_near_duplicates(&to_transcriptions(entries), threshold);
    Ok(pythonize(py, &result)?)
}

#[pyfunction(name = "extract_ngrams")]
fn extract_ngrams_py<'py>(
    py: Python<'py>,
    entries: Vec<(usize, String, String)>,
    min_n: usize,
    max_n: usize,
    min_count: usize,
) -> PyResult<Bound<'py, PyAny>> {
    let result = rust_extract_ngrams(&to_transcriptions(entries), min_n, max_n, min_count);
    Ok(pythonize(py, &result)?)
}

#[pyfunction(name = "find_repeated_sequences")]
fn find_repeated_sequences_py<'py>(
    py: Python<'py>,
    entries: Vec<(usize, String, String)>,
    min_len: usize,
    max_len: usize,
    min_occurrences: usize,
) -> PyResult<Bound<'py, PyAny>> {
    let result =
        rust_find_repeated_sequences(&to_transcriptions(entries), min_len, max_len, min_occurrences);
    Ok(pythonize(py, &result)?)
}

#[pyfunction(name = "find_near_duplicate_sequences")]
fn find_near_duplicate_sequences_py<'py>(
    py: Python<'py>,
    entries: Vec<(usize, String, String)>,
    min_len: usize,
    max_len: usize,
    threshold: f64,
    min_occurrences: usize,
    exact_sequences_json: &str,
) -> PyResult<Bound<'py, PyAny>> {
    // Deserialize exact sequences from JSON
    let raw: Vec<serde_json::Value> = serde_json::from_str(exact_sequences_json)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("invalid JSON: {e}")))?;

    let exact_sequences: Vec<RepeatedSequence> = raw
        .into_iter()
        .map(|v| {
            let length = v["length"].as_u64().unwrap_or(0) as usize;
            let occurrences = v["occurrences"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|o| SequenceOccurrence {
                            start_index: o["start_index"].as_u64().unwrap_or(0) as usize,
                            start_id: o["start_id"].as_str().unwrap_or("").to_string(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            let entry_texts = v["entry_texts"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            RepeatedSequence {
                length,
                occurrences,
                entry_texts,
            }
        })
        .collect();

    let result = rust_find_near_duplicate_sequences(
        &to_transcriptions(entries),
        min_len,
        max_len,
        threshold,
        min_occurrences,
        &exact_sequences,
    );
    Ok(pythonize(py, &result)?)
}

#[pymodule]
#[pyo3(name = "native_helper")]
fn native_helper(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(normalize_py, module)?)?;
    module.add_function(wrap_pyfunction!(find_near_duplicates_py, module)?)?;
    module.add_function(wrap_pyfunction!(extract_ngrams_py, module)?)?;
    module.add_function(wrap_pyfunction!(find_repeated_sequences_py, module)?)?;
    module.add_function(wrap_pyfunction!(find_near_duplicate_sequences_py, module)?)?;
    Ok(())
}
