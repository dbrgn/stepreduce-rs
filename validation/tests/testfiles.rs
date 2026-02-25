use std::{fs, io::Write, path::Path};

use stepreduce::ReduceOptions;
use stepreduce_rs_validation::{
    DEFAULT_ABS_TOL, DEFAULT_REL_TOL, compare_props, compute_props, load_step,
};

fn test_geometric_equivalence(path: &Path) -> datatest_stable::Result<()> {
    // Load original STEP file into OCCT and compute its properties.
    let original_shape = load_step(path);
    let original_props = compute_props(&original_shape);

    // Run stepreduce on the original file.
    let input = fs::read(path)?;
    let reduced_bytes = stepreduce::reduce(&input, &ReduceOptions::default());

    // Write reduced output to a temporary file (OCCT needs a file path).
    let mut tmp = tempfile::NamedTempFile::with_suffix(".step")?;
    tmp.write_all(&reduced_bytes)?;
    tmp.flush()?;

    // Load reduced STEP file into OCCT and compute its properties.
    let reduced_shape = load_step(tmp.path());
    let reduced_props = compute_props(&reduced_shape);

    // Compare.
    let result = compare_props(
        &original_props,
        &reduced_props,
        DEFAULT_REL_TOL,
        DEFAULT_ABS_TOL,
    );

    assert!(
        result.passed,
        "geometric mismatch for {}\nOriginal: {original_props}\nReduced:  {reduced_props}\n{result}",
        path.display(),
    );

    Ok(())
}

datatest_stable::harness! {
    { test = test_geometric_equivalence, root = "testfiles", pattern = r"\.step$" },
}
