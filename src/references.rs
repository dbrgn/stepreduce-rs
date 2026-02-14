use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use regex::Regex;

/// Pattern matching STEP entity references like `#123`.
static REF_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"#(\d+)").unwrap());

/// Collect all entity reference IDs (`#NNN`) from a right-hand side string.
pub(crate) fn collect_references(rhs: &str) -> HashSet<u32> {
    REF_PATTERN
        .captures_iter(rhs)
        .filter_map(|cap| cap[1].parse::<u32>().ok())
        .collect()
}

/// Remap all `#NNN` references in `rhs` according to `lookup`.
///
/// References not present in `lookup` are left unchanged.
pub(crate) fn remap_references(rhs: &str, lookup: &HashMap<u32, u32>) -> String {
    let mut result = String::with_capacity(rhs.len());
    let mut last_pos = 0;

    for m in REF_PATTERN.find_iter(rhs) {
        result.push_str(&rhs[last_pos..m.start()]);

        let old_val: u32 = rhs[m.start() + 1..m.end()].parse().unwrap();

        if let Some(&new_val) = lookup.get(&old_val) {
            result.push('#');
            result.push_str(&new_val.to_string());
        } else {
            result.push_str(m.as_str());
        }

        last_pos = m.end();
    }

    result.push_str(&rhs[last_pos..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    mod collect_references {
        use super::*;

        #[test]
        fn basic() {
            let refs = collect_references("FOO(#1,#2,#3)");
            assert_eq!(refs, HashSet::from([1, 2, 3]));
        }

        #[test]
        fn no_refs() {
            let refs = collect_references("CARTESIAN_POINT('',0.,1.,2.)");
            assert!(refs.is_empty());
        }
    }

    mod remap_references {
        use super::*;

        #[test]
        fn basic() {
            let lookup = HashMap::from([(1, 10), (3, 30)]);
            let result = remap_references("FOO(#1,#2,#3)", &lookup);
            assert_eq!(result, "FOO(#10,#2,#30)");
        }

        #[test]
        fn no_matches() {
            let lookup = HashMap::new();
            let result = remap_references("FOO(#1,#2)", &lookup);
            assert_eq!(result, "FOO(#1,#2)");
        }
    }
}
