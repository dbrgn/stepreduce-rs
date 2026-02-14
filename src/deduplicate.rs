use std::collections::HashMap;

use crate::{
    normalize::{normalize_entity_name, normalize_numbers_in_line},
    references::remap_references,
};

/// Extract the entity type name from a right-hand side string.
///
/// For `PRODUCT('name',#1,#2)` this returns `PRODUCT`.
/// For a bare identifier like `FOO` this returns the trimmed string.
pub(crate) fn get_entity_type(rhs: &str) -> &str {
    let trimmed = rhs.trim();
    match trimmed.find('(') {
        Some(pos) => trimmed[..pos].trim(),
        None => trimmed,
    }
}

/// STEP entity types that carry identity and must never be deduplicated, even
/// if their normalized content is identical.
const IDENTITY_ENTITIES: &[&str] = &[
    "PRODUCT",
    "PRODUCT_DEFINITION",
    "PRODUCT_DEFINITION_FORMATION",
    "PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE",
    "PRODUCT_DEFINITION_SHAPE",
    "PRODUCT_DEFINITION_CONTEXT",
    "PRODUCT_DEFINITION_WITH_ASSOCIATED_DOCUMENTS",
    "PRODUCT_RELATED_PRODUCT_CATEGORY",
    "SHAPE_DEFINITION_REPRESENTATION",
    "SHAPE_REPRESENTATION",
    "SHAPE_REPRESENTATION_RELATIONSHIP",
    "ADVANCED_BREP_SHAPE_REPRESENTATION",
    "MANIFOLD_SOLID_BREP",
    "MANIFOLD_SURFACE_SHAPE_REPRESENTATION",
    "GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION",
    "GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION",
    "STYLED_ITEM",
    "OVER_RIDING_STYLED_ITEM",
    "PRESENTATION_LAYER_ASSIGNMENT",
    "APPLICATION_CONTEXT",
    "APPLICATION_PROTOCOL_DEFINITION",
    "PRODUCT_CONTEXT",
    "DESIGN_CONTEXT",
];

fn is_identity_entity(entity_type: &str) -> bool {
    IDENTITY_ENTITIES.contains(&entity_type)
}

/// Iteratively deduplicate STEP data lines.
///
/// Entities with identical normalized right-hand sides are merged (the
/// duplicate is removed and all references to it are remapped to the
/// surviving entity). Identity-bearing entities are always kept separate.
///
/// The loop repeats until a fixed point is reached (no further merges).
pub(crate) fn deduplicate(data_lines: &[String], max_decimals: Option<u32>) -> Vec<String> {
    let mut out_lines: Vec<String> = data_lines.to_vec();

    loop {
        let in_lines = out_lines;
        let mut uniques: HashMap<String, u32> = HashMap::new();
        let mut lookup: HashMap<u32, u32> = HashMap::new();
        out_lines = Vec::new();

        for line in &in_lines {
            let Some(eq) = line.find('=') else {
                continue;
            };

            let old_num: u32 = line[1..eq].parse().unwrap_or(0);
            let rhs = line[eq + 1..].trim();

            let entity_type = get_entity_type(rhs);

            // Normalize a copy for comparison; keep original for output.
            let mut norm_rhs = normalize_numbers_in_line(rhs, max_decimals);
            norm_rhs = normalize_entity_name(&norm_rhs);

            if is_identity_entity(entity_type) {
                // Force uniqueness for identity-bearing entities.
                while uniques.contains_key(&norm_rhs) {
                    norm_rhs.push(' ');
                }
                let new_id = out_lines.len() as u32 + 1;
                uniques.insert(norm_rhs, new_id);
                lookup.insert(old_num, new_id);
                out_lines.push(format!("#{new_id}={rhs}"));
            } else if let Some(&existing_id) = uniques.get(&norm_rhs) {
                lookup.insert(old_num, existing_id);
            } else {
                let new_id = out_lines.len() as u32 + 1;
                uniques.insert(norm_rhs, new_id);
                lookup.insert(old_num, new_id);
                out_lines.push(format!("#{new_id}={rhs}"));
            }
        }

        // Remap all references.
        for line in &mut out_lines {
            let eq = line.find('=').unwrap();
            let lhs = &line[..eq];
            let rhs = &line[eq + 1..];
            *line = format!("{lhs}={}", remap_references(rhs, &lookup));
        }

        if in_lines.len() <= out_lines.len() {
            break;
        }
    }

    out_lines
}

#[cfg(test)]
mod tests {
    use super::*;

    mod get_entity_type {
        use super::*;

        #[test]
        fn with_parens() {
            assert_eq!(get_entity_type("PRODUCT('foo',#1)"), "PRODUCT");
        }

        #[test]
        fn bare() {
            assert_eq!(get_entity_type("  FOO_BAR  "), "FOO_BAR");
        }
    }

    mod deduplicate {
        #[test]
        fn removes_duplicates() {
            let lines = vec![
                "#1=CARTESIAN_POINT('',0.,0.,0.)".to_string(),
                "#2=CARTESIAN_POINT('',0.,0.,0.)".to_string(),
                "#3=DIRECTION('',1.,0.,0.)".to_string(),
                "#4=AXIS2_PLACEMENT_3D('',#1,#3,#3)".to_string(),
                "#5=AXIS2_PLACEMENT_3D('',#2,#3,#3)".to_string(),
            ];
            let result = super::deduplicate(&lines, None);
            // #2 should be merged into #1, and #5 into #4
            assert!(result.len() < lines.len());
        }

        #[test]
        fn preserves_identity_entities() {
            let lines = vec![
                "#1=PRODUCT('a','a',$,(#3))".to_string(),
                "#2=PRODUCT('b','b',$,(#3))".to_string(),
                "#3=PRODUCT_CONTEXT('',#4,'design')".to_string(),
            ];
            let result = super::deduplicate(&lines, None);
            // Both PRODUCTs should survive (identity entities).
            let product_count = result.iter().filter(|l| l.contains("PRODUCT(")).count();
            assert_eq!(product_count, 2);
        }
    }
}
