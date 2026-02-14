use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

use stepreduce::ReduceOptions;

/// Reduce STEP file size by deduplicating entities and removing orphans.
#[derive(Parser)]
#[command(name = "stepreduce", version)]
struct Cli {
    /// Input STEP file.
    input: PathBuf,

    /// Output STEP file (may be the same as input).
    output: PathBuf,

    /// Print reduction statistics.
    #[arg(short, long)]
    verbose: bool,

    /// Maximum decimal places for numeric comparison.
    #[arg(short, long)]
    precision: Option<u32>,

    /// Derive precision from the STEP file's
    /// UNCERTAINTY_MEASURE_WITH_UNIT value.
    #[arg(long)]
    use_step_precision: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    let options = ReduceOptions {
        verbose: cli.verbose,
        max_decimals: cli.precision,
        use_step_precision: cli.use_step_precision,
    };

    stepreduce::reduce(&cli.input, &cli.output, &options).with_context(|| {
        format!(
            "failed to reduce {} -> {}",
            cli.input.display(),
            cli.output.display()
        )
    })?;

    Ok(())
}
