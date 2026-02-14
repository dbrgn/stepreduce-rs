use std::{fs, path::Path};

use stepreduce::ReduceOptions;

fn test_reduce(path: &Path) -> datatest_stable::Result<()> {
    let input = fs::read(path)?;
    let expected = fs::read(path.with_extension("step.min"))?;
    let actual = stepreduce::reduce(&input, &ReduceOptions::default())?;
    assert_eq!(actual, expected, "mismatch for {}", path.display());
    Ok(())
}

datatest_stable::harness! {
    { test = test_reduce, root = "test-vectors", pattern = r"\.step$" },
}
