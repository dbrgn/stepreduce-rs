use std::collections::{HashMap, HashSet};

use crate::{
    deduplicate::get_entity_type,
    references::{collect_references, remap_references},
};

/// STEP entity types that serve as GC roots. Any entity reachable from one of
/// these (transitively via `#NNN` references) is kept; everything else is
/// removed.
const GC_ROOT_ENTITIES: &[&str] = &[
    "PRODUCT_DEFINITION",
    "APPLICATION_PROTOCOL_DEFINITION",
    "SHAPE_DEFINITION_REPRESENTATION",
    "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION",
    "DRAUGHTING_MODEL",
    "PRESENTATION_LAYER_ASSIGNMENT",
    "APPLICATION_CONTEXT",
];

/// Remove unreachable ("orphan") entities from the data section.
///
/// Starting from entities whose types are in [`GC_ROOT_ENTITIES`], a
/// forward-reference walk marks all transitively reachable entities. Entities
/// not reached are dropped, and surviving entities are renumbered starting
/// from 1.
///
/// If no GC roots are found (e.g. the file has an unusual structure), all
/// lines are returned unchanged.
pub(crate) fn remove_orphans(lines: &[String]) -> Vec<String> {
    let mut id_to_rhs: HashMap<u32, &str> = HashMap::new();
    let mut id_to_refs: HashMap<u32, HashSet<u32>> = HashMap::new();

    for line in lines {
        let Some(eq) = line.find('=') else {
            continue;
        };
        let eid: u32 = match line[1..eq].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        let rhs = &line[eq + 1..];
        id_to_rhs.insert(eid, rhs);
        id_to_refs.insert(eid, collect_references(rhs));
    }

    // Seed the reachable set from GC root entity types.
    let mut reachable: HashSet<u32> = HashSet::new();
    let mut stack: Vec<u32> = Vec::new();

    for (&eid, rhs) in &id_to_rhs {
        let etype = get_entity_type(rhs);
        if GC_ROOT_ENTITIES.contains(&etype) {
            stack.push(eid);
            reachable.insert(eid);
        }
    }

    // Walk forward references.
    while let Some(eid) = stack.pop() {
        if let Some(refs) = id_to_refs.get(&eid) {
            for &r in refs {
                if !reachable.contains(&r) && id_to_rhs.contains_key(&r) {
                    reachable.insert(r);
                    stack.push(r);
                }
            }
        }
    }

    if reachable.is_empty() {
        return lines.to_vec();
    }

    // Rebuild with only reachable entities, renumbered.
    let mut renumber: HashMap<u32, u32> = HashMap::new();
    let mut surviving: Vec<(u32, &str)> = Vec::new();

    for line in lines {
        let Some(eq) = line.find('=') else {
            continue;
        };
        let eid: u32 = match line[1..eq].trim().parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        if reachable.contains(&eid) {
            let new_id = surviving.len() as u32 + 1;
            renumber.insert(eid, new_id);
            surviving.push((new_id, &line[eq + 1..]));
        }
    }

    surviving
        .iter()
        .map(|(new_id, rhs)| {
            let remapped = remap_references(rhs, &renumber);
            format!("#{new_id}={remapped}")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_unreachable() {
        let lines = vec![
            "#1=APPLICATION_CONTEXT('core')".to_string(),
            "#2=PRODUCT_DEFINITION('pd',#1)".to_string(),
            "#3=CARTESIAN_POINT('',0.,0.,0.)".to_string(), // orphan
        ];
        let result = remove_orphans(&lines);
        assert_eq!(result.len(), 2);
        // The orphan CARTESIAN_POINT should be gone.
        assert!(!result.iter().any(|l| l.contains("CARTESIAN_POINT")));
    }

    #[test]
    fn keeps_transitively_reachable() {
        let lines = vec![
            "#1=APPLICATION_CONTEXT('core')".to_string(),
            "#2=PRODUCT_DEFINITION('pd',#3)".to_string(),
            "#3=CARTESIAN_POINT('',0.,0.,0.)".to_string(), // reachable via #2
        ];
        let result = remove_orphans(&lines);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn no_roots_returns_all() {
        let lines = vec![
            "#1=CARTESIAN_POINT('',0.,0.,0.)".to_string(),
            "#2=DIRECTION('',1.,0.,0.)".to_string(),
        ];
        let result = remove_orphans(&lines);
        assert_eq!(result.len(), 2);
    }
}
