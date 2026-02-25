//! Reduce STEP CAD file size by deduplicating entities and removing orphans.
//!
//! This is a lossless, one-way compression / redundancy-reduction algorithm
//! targeted at STEP (ISO 10303-21) files.
//!
//! # Example
//!
//! ```
//! use stepreduce::{ReduceOptions, reduce};
//!
//! let step_data = b"ISO-10303-21;\nHEADER;\nENDSEC;\nDATA;\n#1=FOO('x');\nENDSEC;\nEND-ISO-10303-21;\n";
//! let opts = ReduceOptions::default();
//! let reduced = reduce(step_data, &opts).unwrap();
//! assert!(!reduced.is_empty());
//! ```

use std::io::Write;

mod deduplicate;
mod error;
mod find_numbers;
mod normalize;
mod orphans;
mod parse;
mod references;

pub use crate::error::ReduceError;

/// Options controlling the reduction process.
#[derive(Debug, Clone, Default)]
pub struct ReduceOptions {
    /// Maximum number of decimal places for numeric comparison.
    ///
    /// `None` means no rounding — numbers are only normalized (scientific
    /// notation expanded, redundant zeros stripped).
    pub max_decimals: Option<u32>,

    /// Derive precision from the STEP file's
    /// `UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(…))` value.
    ///
    /// When both this and `max_decimals` are set, the smaller value wins.
    pub use_step_precision: bool,
}

/// Reduce a STEP file by deduplicating entities and removing orphans.
///
/// Accepts raw STEP file content as a byte slice and returns the reduced
/// content as a `Vec<u8>`.
pub fn reduce(input: &[u8], options: &ReduceOptions) -> Result<Vec<u8>, ReduceError> {
    let reader = std::io::Cursor::new(input);
    let parsed = parse::parse_data_section(reader);

    let mut max_decimals = options.max_decimals;

    if options.use_step_precision
        && let Some(step_decimals) = normalize::extract_uncertainty(&parsed.data)
    {
        max_decimals = Some(match max_decimals {
            Some(current) => current.min(step_decimals),
            None => step_decimals,
        });
    }

    let data_lines = deduplicate::deduplicate(&parsed.data, max_decimals);
    let data_lines = orphans::remove_orphans(&data_lines);

    let mut output = Vec::with_capacity(input.len());

    for line in &parsed.header {
        writeln!(output, "{line}")?;
    }
    for line in &data_lines {
        writeln!(output, "{line}")?;
    }
    for line in &parsed.footer {
        writeln!(output, "{line}")?;
    }

    Ok(output)
}
