mod exact;
mod near_sequences;
mod ngrams;
mod sequences;
pub mod similarity;
pub mod types;

pub use exact::{find_exact_duplicates, find_near_duplicates};
pub use near_sequences::find_near_duplicate_sequences;
pub use ngrams::extract_ngrams;
pub use sequences::find_repeated_sequences;
pub use similarity::normalize;

#[cfg(feature = "python")]
mod python;
