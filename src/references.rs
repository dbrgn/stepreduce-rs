use std::collections::{HashMap, HashSet};

/// A single `#NNN` reference match inside a string.
struct RefMatch {
    /// Byte offset of `#` in the source string.
    start: usize,
    /// Byte offset one past the last digit.
    end: usize,
    /// The parsed numeric value.
    value: u32,
}

/// Iterate over all `#NNN` entity references in `s`.
fn ref_matches(s: &str) -> impl Iterator<Item = RefMatch> + '_ {
    let bytes = s.as_bytes();
    let mut pos = 0;

    std::iter::from_fn(move || {
        while pos < bytes.len() {
            if bytes[pos] == b'#' {
                let num_start = pos + 1;
                let mut num_end = num_start;

                while num_end < bytes.len() && bytes[num_end].is_ascii_digit() {
                    num_end += 1;
                }

                if num_end > num_start
                    && let Ok(value) = s[num_start..num_end].parse::<u32>()
                {
                    let m = RefMatch {
                        start: pos,
                        end: num_end,
                        value,
                    };
                    pos = num_end;
                    return Some(m);
                }

                pos = num_end.max(pos + 1);
            } else {
                pos += 1;
            }
        }
        None
    })
}

/// Collect all entity reference IDs (`#NNN`) from a right-hand side string.
pub(crate) fn collect_references(rhs: &str) -> HashSet<u32> {
    ref_matches(rhs).map(|m| m.value).collect()
}

/// Remap all `#NNN` references in `rhs` according to `lookup`.
///
/// References not present in `lookup` are left unchanged.
pub(crate) fn remap_references(rhs: &str, lookup: &HashMap<u32, u32>) -> String {
    let mut result = String::with_capacity(rhs.len());
    let mut last_pos = 0;

    for m in ref_matches(rhs) {
        result.push_str(&rhs[last_pos..m.start]);

        if let Some(&new_val) = lookup.get(&m.value) {
            result.push('#');
            result.push_str(&new_val.to_string());
        } else {
            result.push_str(&rhs[m.start..m.end]);
        }

        last_pos = m.end;
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
