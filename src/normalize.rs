/// A single floating-point number match inside a string.
struct NumMatch {
    /// Byte offset of the first character of the number (may be `-`).
    start: usize,
    /// Byte offset one past the last character of the number.
    end: usize,
}

/// Try to consume an optional exponent (`[eE][+-]?\d+`) starting at `pos`.
/// Returns the new position (unchanged if no exponent found).
fn try_consume_exponent(bytes: &[u8], pos: usize) -> usize {
    if pos >= bytes.len() {
        return pos;
    }

    if bytes[pos] != b'e' && bytes[pos] != b'E' {
        return pos;
    }

    let mut p = pos + 1;

    if p < bytes.len() && (bytes[p] == b'+' || bytes[p] == b'-') {
        p += 1;
    }

    let digit_start = p;

    while p < bytes.len() && bytes[p].is_ascii_digit() {
        p += 1;
    }

    // Must have at least one digit after 'E'/'e' for a valid exponent.
    if p > digit_start { p } else { pos }
}

/// Find all floating-point number literals in `s`, respecting the guard that
/// the character before the match must not be `[A-Za-z_#]` (or the match is at
/// the start of the string).
///
/// The matched forms are:
/// - `-?\d+\.\d*([eE][+-]?\d+)?`  (e.g. `1.0`, `-3.14`, `1.0E-3`)
/// - `-?\d+[eE][+-]?\d+`           (integer with exponent, e.g. `2E+5`)
/// - `-?\.\d+([eE][+-]?\d+)?`      (leading dot, e.g. `.5`)
fn find_numbers(s: &str) -> impl Iterator<Item = NumMatch> + '_ {
    let bytes = s.as_bytes();
    let mut pos: usize = 0;

    std::iter::from_fn(move || {
        while pos < bytes.len() {
            // Check the guard: preceding char must not be [A-Za-z_#], or we
            // must be at the start of the string.
            let guard_ok = pos == 0 || {
                let prev = bytes[pos - 1];
                !prev.is_ascii_alphabetic() && prev != b'_' && prev != b'#'
            };

            if !guard_ok {
                pos += 1;
                continue;
            }

            let start = pos;
            let mut p = pos;

            // Optional leading minus.
            let has_minus = p < bytes.len() && bytes[p] == b'-';
            if has_minus {
                p += 1;
            }

            // Try to match digits before a dot or exponent.
            let digits_start = p;

            while p < bytes.len() && bytes[p].is_ascii_digit() {
                p += 1;
            }

            let has_leading_digits = p > digits_start;

            if has_leading_digits {
                if p < bytes.len() && bytes[p] == b'.' {
                    // Form 1: `-?\d+\.\d*([eE][+-]?\d+)?`
                    p += 1; // consume dot

                    while p < bytes.len() && bytes[p].is_ascii_digit() {
                        p += 1;
                    }

                    p = try_consume_exponent(bytes, p);

                    let m = NumMatch { start, end: p };
                    pos = p;
                    return Some(m);
                } else if p < bytes.len() && (bytes[p] == b'e' || bytes[p] == b'E') {
                    // Form 2: `-?\d+[eE][+-]?\d+` (mandatory exponent)
                    let after_exp = try_consume_exponent(bytes, p);

                    if after_exp > p {
                        let m = NumMatch {
                            start,
                            end: after_exp,
                        };
                        pos = after_exp;
                        return Some(m);
                    }
                }
                // Bare integer with no dot and no exponent — not a float match.
            } else if p < bytes.len() && bytes[p] == b'.' {
                // Form 3: `-?\.\d+([eE][+-]?\d+)?` — must have digits after dot.
                p += 1; // consume dot
                let frac_start = p;

                while p < bytes.len() && bytes[p].is_ascii_digit() {
                    p += 1;
                }

                if p > frac_start {
                    p = try_consume_exponent(bytes, p);

                    let m = NumMatch { start, end: p };
                    pos = p;
                    return Some(m);
                }
            }

            // No match starting here; advance.
            // If we consumed a minus but didn't match, step past just the minus.
            pos = if has_minus && !has_leading_digits {
                start + 1
            } else if has_leading_digits {
                // We had digits but no dot/exponent — skip past them so we
                // don't re-examine each digit.
                p
            } else {
                start + 1
            };
        }

        None
    })
}

/// Normalize a single floating-point number string into a canonical form.
///
/// Scientific notation is expanded, leading zeros on the integer part and
/// trailing zeros on the fractional part are stripped, and the result always
/// contains a decimal point.
pub(crate) fn normalize_number(s: &str) -> String {
    let mut exp_val: i32 = 0;
    let mantissa_str;

    // Split off exponent.
    let upper = s.to_ascii_uppercase();
    if let Some(e_pos) = upper.find('E') {
        mantissa_str = &s[..e_pos];
        exp_val = s[e_pos + 1..].parse::<i32>().unwrap_or(0);
    } else {
        mantissa_str = s;
    }

    let negative = mantissa_str.starts_with('-');
    let mantissa_str = if negative {
        &mantissa_str[1..]
    } else {
        mantissa_str
    };

    let (mut int_part, mut frac_part);
    if let Some(dot) = mantissa_str.find('.') {
        int_part = mantissa_str[..dot].to_string();
        frac_part = mantissa_str[dot + 1..].to_string();
    } else {
        int_part = mantissa_str.to_string();
        frac_part = String::new();
    }

    if int_part.is_empty() {
        int_part = "0".to_string();
    }

    // Apply exponent by shifting the decimal point.
    if exp_val > 0 {
        let shift = exp_val as usize;
        if shift < frac_part.len() {
            int_part.push_str(&frac_part[..shift]);
            frac_part = frac_part[shift..].to_string();
        } else {
            int_part.push_str(&frac_part);
            int_part.push_str(&"0".repeat(shift - frac_part.len()));
            frac_part.clear();
        }
    } else if exp_val < 0 {
        let shift = (-exp_val) as usize;
        if shift < int_part.len() {
            let split = int_part.len() - shift;
            frac_part = format!("{}{}", &int_part[split..], frac_part);
            int_part = int_part[..split].to_string();
        } else {
            frac_part = format!(
                "{}{}{}",
                "0".repeat(shift - int_part.len()),
                int_part,
                frac_part
            );
            int_part = "0".to_string();
        }
    }

    // Strip leading zeros from integer part.
    let first_nonzero = int_part.find(|c: char| c != '0');
    match first_nonzero {
        None => int_part = "0".to_string(),
        Some(pos) => int_part = int_part[pos..].to_string(),
    }

    // Strip trailing zeros from fractional part.
    let last_nonzero = frac_part.rfind(|c: char| c != '0');
    match last_nonzero {
        None => frac_part.clear(),
        Some(pos) => frac_part = frac_part[..=pos].to_string(),
    }

    // Detect negative zero.
    if int_part == "0" && frac_part.is_empty() {
        return "0.".to_string();
    }

    let result = format!("{int_part}.{frac_part}");
    if negative {
        format!("-{result}")
    } else {
        result
    }
}

/// Round a number string to at most `max_decimals` fractional digits, then
/// normalize.
pub(crate) fn round_number(s: &str, max_decimals: u32) -> String {
    let normalized = normalize_number(s);

    if !normalized.contains('.') {
        return normalized;
    }

    let negative = normalized.starts_with('-');
    let body = if negative {
        &normalized[1..]
    } else {
        &normalized
    };

    let dot = body.find('.').unwrap();
    let int_part = &body[..dot];
    let frac_part = &body[dot + 1..];

    let frac_part = if frac_part.len() > max_decimals as usize {
        &frac_part[..max_decimals as usize]
    } else {
        frac_part
    };

    // Strip trailing zeros.
    let last_nonzero = frac_part.rfind(|c: char| c != '0');
    let frac_part = match last_nonzero {
        None => "",
        Some(pos) => &frac_part[..=pos],
    };

    if int_part == "0" && frac_part.is_empty() {
        return "0.".to_string();
    }

    let result = format!("{int_part}.{frac_part}");
    if negative {
        format!("-{result}")
    } else {
        result
    }
}

/// Replace all floating-point numbers in `rhs` with their normalized (and
/// optionally rounded) forms.
///
/// If `max_decimals` is `Some(n)`, numbers are rounded to `n` decimal places.
/// If `None`, numbers are only normalized (scientific notation expanded, zeros
/// stripped).
pub(crate) fn normalize_numbers_in_line(rhs: &str, max_decimals: Option<u32>) -> String {
    let mut result = String::with_capacity(rhs.len());
    let mut last_pos = 0;

    for m in find_numbers(rhs) {
        result.push_str(&rhs[last_pos..m.start]);

        let num_str = &rhs[m.start..m.end];
        let replacement = match max_decimals {
            Some(n) => round_number(num_str, n),
            None => normalize_number(num_str),
        };
        result.push_str(&replacement);

        last_pos = m.end;
    }

    result.push_str(&rhs[last_pos..]);
    result
}

/// Strip the quoted name from entity declarations like `PRODUCT('name'…`
/// by replacing the name with an empty string.
///
/// Matches the pattern `^[A-Z_]+\('[^']*'` and replaces the quoted content
/// with an empty string.
pub(crate) fn normalize_entity_name(rhs: &str) -> String {
    let bytes = rhs.as_bytes();
    let mut p = 0;

    // Consume `[A-Z_]+`.
    while p < bytes.len() && (bytes[p].is_ascii_uppercase() || bytes[p] == b'_') {
        p += 1;
    }

    if p == 0 {
        return rhs.to_string();
    }

    // Expect `(`.
    if p >= bytes.len() || bytes[p] != b'(' {
        return rhs.to_string();
    }

    let prefix_end = p + 1; // position after '('

    // Expect `'`.
    if prefix_end >= bytes.len() || bytes[prefix_end] != b'\'' {
        return rhs.to_string();
    }

    // Find closing `'`.
    let quote_start = prefix_end;
    let mut q = quote_start + 1;

    while q < bytes.len() && bytes[q] != b'\'' {
        q += 1;
    }

    if q >= bytes.len() {
        return rhs.to_string();
    }

    let quote_end = q + 1; // position after closing '

    // Build: prefix (including '(') + '' + rest after closing quote
    let mut result = String::with_capacity(rhs.len());
    result.push_str(&rhs[..prefix_end]);
    result.push_str("''");
    result.push_str(&rhs[quote_end..]);
    result
}

/// Skip ASCII whitespace starting at `pos`, returning the new position.
fn skip_whitespace(bytes: &[u8], mut pos: usize) -> usize {
    while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    pos
}

/// Check whether `bytes[pos..]` starts with `needle` (case-insensitive ASCII).
/// Returns the position after the needle if matched, or `None`.
fn match_keyword_ci(bytes: &[u8], pos: usize, needle: &[u8]) -> Option<usize> {
    if pos + needle.len() > bytes.len() {
        return None;
    }

    for (i, &expected) in needle.iter().enumerate() {
        if !bytes[pos + i].eq_ignore_ascii_case(&expected) {
            return None;
        }
    }

    Some(pos + needle.len())
}

/// Derive the number of significant decimal places from the STEP file's
/// `UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(<value>))` declarations.
///
/// Returns `Some(n)` where `n` is `ceil(-log10(value)) + 1`, or `None` if
/// no valid uncertainty is found.
pub(crate) fn extract_uncertainty(data_lines: &[String]) -> Option<u32> {
    for line in data_lines {
        let bytes = line.as_bytes();

        // Scan for "UNCERTAINTY_MEASURE_WITH_UNIT" anywhere in the line.
        let keyword = b"UNCERTAINTY_MEASURE_WITH_UNIT";

        for start in 0..bytes.len() {
            let Some(mut p) = match_keyword_ci(bytes, start, keyword) else {
                continue;
            };

            // \s*\(\s*
            p = skip_whitespace(bytes, p);
            if p >= bytes.len() || bytes[p] != b'(' {
                continue;
            }
            p = skip_whitespace(bytes, p + 1);

            // LENGTH_MEASURE
            let Some(after_lm) = match_keyword_ci(bytes, p, b"LENGTH_MEASURE") else {
                continue;
            };
            p = after_lm;

            // \s*\(\s*
            p = skip_whitespace(bytes, p);
            if p >= bytes.len() || bytes[p] != b'(' {
                continue;
            }
            p = skip_whitespace(bytes, p + 1);

            // Capture everything up to ')'.
            let val_start = p;
            while p < bytes.len() && bytes[p] != b')' {
                p += 1;
            }

            if p >= bytes.len() {
                continue;
            }

            let val_str = line[val_start..p].trim();

            if let Ok(val) = val_str.parse::<f64>()
                && val > 0.0
            {
                return Some((-val.log10()).ceil() as u32 + 1);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    mod normalize_number {
        use super::*;

        #[test]
        fn simple() {
            assert_eq!(normalize_number("1.0"), "1.");
            assert_eq!(normalize_number("3.14"), "3.14");
            assert_eq!(normalize_number("-0.5"), "-0.5");
        }

        #[test]
        fn scientific() {
            assert_eq!(normalize_number("1.0E-3"), "0.001");
            assert_eq!(normalize_number("2.5e+2"), "250.");
        }

        #[test]
        fn leading_dot() {
            assert_eq!(normalize_number(".5"), "0.5");
        }

        #[test]
        fn negative_zero() {
            assert_eq!(normalize_number("-0.0"), "0.");
        }

        #[test]
        fn trailing_zeros() {
            assert_eq!(normalize_number("1.200"), "1.2");
        }
    }

    mod round_number {
        use super::*;

        #[test]
        fn truncation() {
            assert_eq!(round_number("3.14159", 3), "3.141");
            assert_eq!(round_number("3.14159", 0), "3.");
        }

        #[test]
        fn shorter_than_limit() {
            assert_eq!(round_number("3.14", 5), "3.14");
        }
    }

    mod normalize_numbers_in_line {
        use super::*;

        #[test]
        fn basic() {
            let input = "CARTESIAN_POINT('',-1.200E+1,3.0,0.00)";
            let result = normalize_numbers_in_line(input, None);
            assert_eq!(result, "CARTESIAN_POINT('',-12.,3.,0.)");
        }

        #[test]
        fn with_rounding() {
            let input = "CARTESIAN_POINT('',1.23456,7.89012)";
            let result = normalize_numbers_in_line(input, Some(3));
            assert_eq!(result, "CARTESIAN_POINT('',1.234,7.89)");
        }
    }

    mod normalize_entity_name {
        use super::*;

        #[test]
        fn strips_name() {
            let input = "PRODUCT('My Part',extra)";
            let result = normalize_entity_name(input);
            assert_eq!(result, "PRODUCT('',extra)");
        }

        #[test]
        fn no_match() {
            let input = "CARTESIAN_POINT('',0.,0.,0.)";
            let result = normalize_entity_name(input);
            assert_eq!(result, input);
        }
    }

    mod extract_uncertainty {
        use super::*;

        #[test]
        fn found() {
            let lines = vec!["UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.001))".to_string()];
            assert_eq!(extract_uncertainty(&lines), Some(4));
        }

        #[test]
        fn not_found() {
            let lines = vec!["CARTESIAN_POINT('',0.,0.,0.)".to_string()];
            assert_eq!(extract_uncertainty(&lines), None);
        }
    }
}
