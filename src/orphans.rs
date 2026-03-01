use std::collections::{HashMap, HashSet};

use crate::{
    deduplicate::get_entity_type,
    references::{collect_references, remap_references},
};

/// STEP entity types that serve as GC roots. Any entity reachable from one of
/// these (transitively via `#NNN` references) is kept; everything else is
/// removed.
const GC_ROOT_ENTITIES: &[&str] = &[
    "APPLICATION_CONTEXT",
    "APPLICATION_PROTOCOL_DEFINITION",
    "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION",
    "DRAUGHTING_MODEL",
    "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION",
    "PRESENTATION_LAYER_ASSIGNMENT",
    "PRODUCT_DEFINITION",
    "SHAPE_DEFINITION_REPRESENTATION",
    "SHAPE_REPRESENTATION_RELATIONSHIP",
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

    #[test]
    fn keeps_shape_representation_relationship_as_root() {
        // SHAPE_REPRESENTATION_RELATIONSHIP is a top-level structural link
        // between a SHAPE_REPRESENTATION and an
        // ADVANCED_BREP_SHAPE_REPRESENTATION. Nothing references it by ID;
        // it must be a GC root so the BREP subtree is preserved.
        let lines = vec![
            "#1=SHAPE_DEFINITION_REPRESENTATION(#2,#3)".to_string(),
            "#2=PRODUCT_DEFINITION_SHAPE('','',#10)".to_string(),
            "#3=SHAPE_REPRESENTATION('',(#11),#12)".to_string(),
            "#4=SHAPE_REPRESENTATION_RELATIONSHIP('','',#3,#5)".to_string(),
            "#5=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#6),#12)".to_string(),
            "#6=MANIFOLD_SOLID_BREP('',#7)".to_string(),
            "#7=CLOSED_SHELL('',(#8))".to_string(),
            "#8=ADVANCED_FACE('',(#9),#13,.T.)".to_string(),
            "#9=FACE_BOUND('',#14,.T.)".to_string(),
            "#10=PRODUCT_DEFINITION('pd','',#15,#16)".to_string(),
            "#11=AXIS2_PLACEMENT_3D('',#17,#18,#19)".to_string(),
            "#12=REPRESENTATION_CONTEXT('','')".to_string(),
            "#13=PLANE('',#11)".to_string(),
            "#14=EDGE_LOOP('',(#20))".to_string(),
            "#15=PRODUCT_DEFINITION_FORMATION('','',#21)".to_string(),
            "#16=APPLICATION_CONTEXT('core')".to_string(),
            "#17=CARTESIAN_POINT('',0.,0.,0.)".to_string(),
            "#18=DIRECTION('',0.,0.,1.)".to_string(),
            "#19=DIRECTION('',1.,0.,0.)".to_string(),
            "#20=ORIENTED_EDGE('',*,*,#22,.T.)".to_string(),
            "#21=PRODUCT('p','','',(#23))".to_string(),
            "#22=EDGE_CURVE('',#24,#24,#25,.T.)".to_string(),
            "#23=PRODUCT_CONTEXT('',#16,'')".to_string(),
            "#24=VERTEX_POINT('',#17)".to_string(),
            "#25=LINE('',#17,#26)".to_string(),
            "#26=VECTOR('',#18,1.)".to_string(),
        ];
        let result = remove_orphans(&lines);
        // The ADVANCED_BREP_SHAPE_REPRESENTATION subtree must survive
        // because SHAPE_REPRESENTATION_RELATIONSHIP is a GC root.
        assert!(
            result
                .iter()
                .any(|l| l.contains("ADVANCED_BREP_SHAPE_REPRESENTATION")),
            "ADVANCED_BREP_SHAPE_REPRESENTATION must be kept via SHAPE_REPRESENTATION_RELATIONSHIP root"
        );
        assert!(
            result.iter().any(|l| l.contains("MANIFOLD_SOLID_BREP")),
            "MANIFOLD_SOLID_BREP must be reachable from SHAPE_REPRESENTATION_RELATIONSHIP"
        );
        assert_eq!(result.len(), lines.len());
    }
}
