//! Reduce STEP CAD file size by deduplicating entities and removing orphans.
//!
//! This is a lossless, one-way compression / redundancy-reduction algorithm
//! targeted at STEP (ISO 10303-21) files.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//!
//! use stepreduce::{ReduceOptions, reduce};
//!
//! let opts = ReduceOptions::default();
//! reduce(Path::new("input.stp"), Path::new("output.stp"), &opts).unwrap();
//! ```

use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

mod deduplicate;
mod error;
mod normalize;
mod orphans;
mod parse;
mod references;

pub use crate::error::ReduceError;

/// Options controlling the reduction process.
#[derive(Debug, Clone, Default)]
pub struct ReduceOptions {
    /// Print reduction statistics via `log::info!`.
    pub verbose: bool,

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
/// Reads from `input`, writes the reduced file to `output`. The two paths may
/// refer to the same file (the input is fully read before any output is
/// written).
pub fn reduce(input: &Path, output: &Path, options: &ReduceOptions) -> Result<(), ReduceError> {
    // Count lines for verbose output (requires a separate pass).
    let n_lines = if options.verbose {
        let file = File::open(input)?;
        BufReader::new(file).lines().count()
    } else {
        0
    };

    let file = File::open(input)?;
    let reader = BufReader::new(file);
    let parsed = parse::parse_data_section(reader);

    let mut max_decimals = options.max_decimals;

    if options.use_step_precision
        && let Some(step_decimals) = normalize::extract_uncertainty(&parsed.data)
    {
        if options.verbose {
            log::info!("derived {step_decimals} decimal places from STEP uncertainty");
        }

        max_decimals = Some(match max_decimals {
            Some(current) => current.min(step_decimals),
            None => step_decimals,
        });
    }

    let data_lines = deduplicate::deduplicate(&parsed.data, max_decimals);
    let data_lines = orphans::remove_orphans(&data_lines);

    let out_file = File::create(output)?;
    let mut writer = BufWriter::new(out_file);

    for line in &parsed.header {
        writeln!(writer, "{line}")?;
    }
    for line in &data_lines {
        writeln!(writer, "{line}")?;
    }
    for line in &parsed.footer {
        writeln!(writer, "{line}")?;
    }

    writer.flush()?;

    if options.verbose {
        let out_total = data_lines.len() + parsed.header.len() + parsed.footer.len();
        log::info!("{} {n_lines} shrunk to {out_total}", input.display());
    }

    Ok(())
}
