//! Geometric validation for STEP file reduction.
//!
//! Uses OpenCASCADE (via `opencascade-sys`) to load STEP files and compare
//! geometric properties (volume, surface area, bounding box, center of mass)
//! between the original and reduced versions. This ensures that the reduction
//! process does not alter the geometry.

use std::{fmt, path::Path};

use opencascade_sys::ffi;

/// Geometric properties extracted from a STEP file via OpenCASCADE.
#[derive(Debug, Clone)]
pub struct GeometricProps {
    /// Volume of all solids (with density 1).
    pub volume: f64,
    /// Total surface area of all faces (with density 1).
    pub surface_area: f64,
    /// Center of mass (from volume properties).
    pub center_of_mass: [f64; 3],
    /// Axis-aligned bounding box minimum corner.
    pub bbox_min: [f64; 3],
    /// Axis-aligned bounding box maximum corner.
    pub bbox_max: [f64; 3],
}

impl fmt::Display for GeometricProps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "volume={:.6}, area={:.6}, CoM=({:.6}, {:.6}, {:.6}), \
             bbox=({:.6}, {:.6}, {:.6})..({:.6}, {:.6}, {:.6})",
            self.volume,
            self.surface_area,
            self.center_of_mass[0],
            self.center_of_mass[1],
            self.center_of_mass[2],
            self.bbox_min[0],
            self.bbox_min[1],
            self.bbox_min[2],
            self.bbox_max[0],
            self.bbox_max[1],
            self.bbox_max[2],
        )
    }
}

/// Load a STEP file from disk and return the combined shape.
///
/// # Panics
///
/// Panics if the file cannot be read or contains no transferable shapes.
pub fn load_step(path: &Path) -> cxx::UniquePtr<ffi::TopoDS_Shape> {
    let path_str = path.to_str().expect("path must be valid UTF-8");

    let mut reader = ffi::STEPControl_Reader_ctor();
    let progress = ffi::Message_ProgressRange_ctor();

    let status = ffi::read_step(reader.pin_mut(), path_str.to_owned());
    assert!(
        matches!(status, ffi::IFSelect_ReturnStatus::IFSelect_RetDone),
        "failed to read STEP file: {path_str} (status: {status:?})",
    );

    let n_roots = reader.pin_mut().TransferRoots(&progress);
    assert!(
        n_roots > 0,
        "no transferable roots in STEP file: {path_str}"
    );

    ffi::one_shape_step(&reader)
}

/// Compute geometric properties of a shape.
pub fn compute_props(shape: &ffi::TopoDS_Shape) -> GeometricProps {
    // Volume properties (also gives center of mass).
    let mut vol_props = ffi::GProp_GProps_ctor();
    ffi::BRepGProp_VolumeProperties(shape, vol_props.pin_mut());
    let volume = vol_props.Mass();
    let com = ffi::GProp_GProps_CentreOfMass(&vol_props);

    // Surface area.
    let mut surf_props = ffi::GProp_GProps_ctor();
    ffi::BRepGProp_SurfaceProperties(shape, surf_props.pin_mut());
    let surface_area = surf_props.Mass();

    // Bounding box.
    let mut bbox = ffi::Bnd_Box_ctor();
    ffi::BRepBndLib_Add(shape, bbox.pin_mut(), false);
    let corner_min = ffi::Bnd_Box_CornerMin(&bbox);
    let corner_max = ffi::Bnd_Box_CornerMax(&bbox);

    GeometricProps {
        volume,
        surface_area,
        center_of_mass: [com.X(), com.Y(), com.Z()],
        bbox_min: [corner_min.X(), corner_min.Y(), corner_min.Z()],
        bbox_max: [corner_max.X(), corner_max.Y(), corner_max.Z()],
    }
}

/// Result of comparing two sets of geometric properties.
#[derive(Debug)]
pub struct ComparisonResult {
    pub passed: bool,
    pub details: Vec<String>,
}

impl fmt::Display for ComparisonResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in &self.details {
            writeln!(f, "  {line}")?;
        }
        Ok(())
    }
}

/// Compare two sets of geometric properties within the given tolerances.
///
/// - `rel_tol`: relative tolerance for volume and surface area (e.g., 1e-4
///   means 0.01%).
/// - `abs_tol`: absolute tolerance for bounding box and center of mass
///   coordinates.
pub fn compare_props(
    original: &GeometricProps,
    reduced: &GeometricProps,
    rel_tol: f64,
    abs_tol: f64,
) -> ComparisonResult {
    let mut passed = true;
    let mut details = Vec::new();

    // Volume comparison (relative).
    let vol_denom = original.volume.abs().max(1e-15);
    let vol_rel = (original.volume - reduced.volume).abs() / vol_denom;
    if vol_rel > rel_tol {
        passed = false;
        details.push(format!(
            "FAIL volume: {:.6} vs {:.6} (rel_err={:.2e}, tol={:.2e})",
            original.volume, reduced.volume, vol_rel, rel_tol,
        ));
    } else {
        details.push(format!(
            "OK   volume: {:.6} vs {:.6} (rel_err={:.2e})",
            original.volume, reduced.volume, vol_rel,
        ));
    }

    // Surface area comparison (relative).
    let area_denom = original.surface_area.abs().max(1e-15);
    let area_rel = (original.surface_area - reduced.surface_area).abs() / area_denom;
    if area_rel > rel_tol {
        passed = false;
        details.push(format!(
            "FAIL area: {:.6} vs {:.6} (rel_err={:.2e}, tol={:.2e})",
            original.surface_area, reduced.surface_area, area_rel, rel_tol,
        ));
    } else {
        details.push(format!(
            "OK   area: {:.6} vs {:.6} (rel_err={:.2e})",
            original.surface_area, reduced.surface_area, area_rel,
        ));
    }

    // Center of mass comparison (absolute).
    for (i, axis) in ["X", "Y", "Z"].iter().enumerate() {
        let diff = (original.center_of_mass[i] - reduced.center_of_mass[i]).abs();
        if diff > abs_tol {
            passed = false;
            details.push(format!(
                "FAIL CoM {axis}: {:.6} vs {:.6} (diff={:.2e}, tol={:.2e})",
                original.center_of_mass[i], reduced.center_of_mass[i], diff, abs_tol,
            ));
        } else {
            details.push(format!(
                "OK   CoM {axis}: {:.6} vs {:.6} (diff={:.2e})",
                original.center_of_mass[i], reduced.center_of_mass[i], diff,
            ));
        }
    }

    // Bounding box comparison (absolute).
    for (i, axis) in ["X", "Y", "Z"].iter().enumerate() {
        let diff_min = (original.bbox_min[i] - reduced.bbox_min[i]).abs();
        let diff_max = (original.bbox_max[i] - reduced.bbox_max[i]).abs();
        if diff_min > abs_tol {
            passed = false;
            details.push(format!(
                "FAIL bbox_min {axis}: {:.6} vs {:.6} (diff={:.2e}, tol={:.2e})",
                original.bbox_min[i], reduced.bbox_min[i], diff_min, abs_tol,
            ));
        }
        if diff_max > abs_tol {
            passed = false;
            details.push(format!(
                "FAIL bbox_max {axis}: {:.6} vs {:.6} (diff={:.2e}, tol={:.2e})",
                original.bbox_max[i], reduced.bbox_max[i], diff_max, abs_tol,
            ));
        }
    }

    ComparisonResult { passed, details }
}

/// Default relative tolerance for volume/area comparisons.
pub const DEFAULT_REL_TOL: f64 = 1e-4;

/// Default absolute tolerance for coordinate comparisons (bbox, center of mass).
pub const DEFAULT_ABS_TOL: f64 = 1e-3;
