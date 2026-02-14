use std::sync::LazyLock;

use regex::Regex;

/// Matches float literals: `1.0`, `-3.14`, `.5`, `1.0E-3`, `-2E+5`, etc.
///
/// The leading character class prevents matching inside identifiers or entity
/// refs. Capture group 1 is the numeric value.
static NUM_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:[^A-Za-z_#]|^)(-?\d+\.\d*(?:[eE][+-]?\d+)?|-?\d+[eE][+-]?\d+|-?\.\d+(?:[eE][+-]?\d+)?)",
    )
    .unwrap()
});

/// Matches entity declarations like `PRODUCT('name'` and captures up to and
/// including the opening quote.
static NAME_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Z_]+\()'[^']*'").unwrap());

/// Matches `UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(<value>)` and
/// captures `<value>`.
static UNCERTAINTY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)UNCERTAINTY_MEASURE_WITH_UNIT\s*\(\s*LENGTH_MEASURE\s*\(\s*([^)]+)\s*\)")
        .unwrap()
});

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

    for caps in NUM_PATTERN.captures_iter(rhs) {
        let m = caps.get(1).unwrap();
        result.push_str(&rhs[last_pos..m.start()]);

        let num_str = m.as_str();
        let replacement = match max_decimals {
            Some(n) => round_number(num_str, n),
            None => normalize_number(num_str),
        };
        result.push_str(&replacement);

        last_pos = m.end();
    }

    result.push_str(&rhs[last_pos..]);
    result
}

/// Strip the quoted name from entity declarations like `PRODUCT('name'…`
/// by replacing the name with an empty string.
pub(crate) fn normalize_entity_name(rhs: &str) -> String {
    if let Some(caps) = NAME_PATTERN.captures(rhs) {
        let full = caps.get(0).unwrap();
        let prefix = &caps[1]; // e.g. "PRODUCT("
        format!("{}''{}", prefix, &rhs[full.end()..])
    } else {
        rhs.to_string()
    }
}

/// Derive the number of significant decimal places from the STEP file's
/// `UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(<value>))` declarations.
///
/// Returns `Some(n)` where `n` is `ceil(-log10(value)) + 1`, or `None` if
/// no valid uncertainty is found.
pub(crate) fn extract_uncertainty(data_lines: &[String]) -> Option<u32> {
    for line in data_lines {
        if let Some(caps) = UNCERTAINTY_PATTERN.captures(line)
            && let Ok(val) = caps[1].trim().parse::<f64>()
            && val > 0.0
        {
            return Some((-val.log10()).ceil() as u32 + 1);
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
