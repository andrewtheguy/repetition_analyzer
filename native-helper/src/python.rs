use pyo3::prelude::*;
use pythonize::pythonize;

use crate::types::Transcription;
use crate::{
    extract_ngrams as rust_extract_ngrams,
    find_exact_duplicates as rust_find_exact_duplicates,
    find_near_duplicate_sequences as rust_find_near_duplicate_sequences,
    find_near_duplicates as rust_find_near_duplicates,
    find_repeated_sequences as rust_find_repeated_sequences,
};

#[pyfunction(name = "find_exact_duplicates")]
fn find_exact_duplicates_py<'py>(
    py: Python<'py>,
    entries: Vec<Transcription>,
) -> PyResult<Bound<'py, PyAny>> {
    let result = rust_find_exact_duplicates(&entries);
    Ok(pythonize(py, &result)?)
}

#[pyfunction(name = "find_near_duplicates")]
fn find_near_duplicates_py<'py>(
    py: Python<'py>,
    entries: Vec<Transcription>,
    threshold: f64,
) -> PyResult<Bound<'py, PyAny>> {
    let result = rust_find_near_duplicates(&entries, threshold);
    Ok(pythonize(py, &result)?)
}

#[pyfunction(name = "extract_ngrams")]
fn extract_ngrams_py<'py>(
    py: Python<'py>,
    entries: Vec<Transcription>,
    min_n: usize,
    max_n: usize,
    min_count: usize,
) -> PyResult<Bound<'py, PyAny>> {
    let result = rust_extract_ngrams(&entries, min_n, max_n, min_count);
    Ok(pythonize(py, &result)?)
}

#[pyfunction(name = "find_all_sequences")]
fn find_all_sequences_py<'py>(
    py: Python<'py>,
    entries: Vec<Transcription>,
    min_len: usize,
    max_len: usize,
    min_occurrences: usize,
    similarity_threshold: f64,
) -> PyResult<(Bound<'py, PyAny>, Bound<'py, PyAny>)> {
    let exact_sequences =
        rust_find_repeated_sequences(&entries, min_len, max_len, min_occurrences);
    let near_sequences = rust_find_near_duplicate_sequences(
        &entries,
        min_len,
        max_len,
        similarity_threshold,
        min_occurrences,
        &exact_sequences,
    );
    Ok((
        pythonize(py, &exact_sequences)?,
        pythonize(py, &near_sequences)?,
    ))
}

#[pymodule]
#[pyo3(name = "native_helper")]
fn native_helper(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(find_exact_duplicates_py, module)?)?;
    module.add_function(wrap_pyfunction!(find_near_duplicates_py, module)?)?;
    module.add_function(wrap_pyfunction!(extract_ngrams_py, module)?)?;
    module.add_function(wrap_pyfunction!(find_all_sequences_py, module)?)?;
    Ok(())
}
