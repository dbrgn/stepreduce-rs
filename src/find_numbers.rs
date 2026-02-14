//! Hand-written float-literal scanner, as a replacement for the original NUM_PATTERN regex. Since the regex
//! is used in a hot loop, this optimization results in a significant speed gain.
//!
//! The function matches the same three forms the original regex did:
//!   1. `-?\d+\.\d*([eE][+-]?\d+)?`    e.g.  1.0   -3.14   1.0E-3
//!   2. `-?\d+[eE][+-]?\d+`            e.g.  2E+5
//!   3. `-?\.\d+([eE][+-]?\d+)?`       e.g.  .5
//!
//! Guard: the character immediately before the match must NOT be [A-Za-z_#] (prevents matching inside
//! identifiers or entity references).

/// A matched float literal inside a string (byte offsets).
pub(crate) struct NumMatch {
    pub start: usize,
    pub end: usize,
}

/// Return `true` if `ch` can appear immediately before a float literal.
/// The original regex required `[^A-Za-z_#]` or start-of-string.
fn is_valid_num_predecessor(ch: u8) -> bool {
    !ch.is_ascii_alphabetic() && ch != b'_' && ch != b'#'
}

/// Starting at `bytes[pos]`, consume as many ASCII digits as possible.
/// Returns the position after the last digit.
fn eat_digits(bytes: &[u8], pos: usize) -> usize {
    let mut p = pos;
    while p < bytes.len() && bytes[p].is_ascii_digit() {
        p += 1;
    }
    p
}

/// Starting at `bytes[pos]`, try to consume `[eE][+-]?\d+`.
/// Returns the new position, or the original `pos` if no exponent was found.
fn eat_exponent(bytes: &[u8], pos: usize) -> usize {
    if pos >= bytes.len() || (bytes[pos] != b'e' && bytes[pos] != b'E') {
        return pos;
    }
    let mut p = pos + 1;
    if p < bytes.len() && (bytes[p] == b'+' || bytes[p] == b'-') {
        p += 1;
    }
    let after = eat_digits(bytes, p);
    if after > p { after } else { pos } // need ≥1 digit
}

/// Try to parse a float literal starting at `bytes[start]`.
/// Returns `Some(end)` on success, where `end` is one past the last byte.
fn try_match_number(bytes: &[u8], start: usize) -> Option<usize> {
    let mut p = start;

    // Optional leading minus.
    if p < bytes.len() && bytes[p] == b'-' {
        p += 1;
    }

    let digits_start = p;
    p = eat_digits(bytes, p);
    let has_digits = p > digits_start;

    if has_digits {
        if p < bytes.len() && bytes[p] == b'.' {
            // Form 1: digits DOT optional-digits optional-exponent
            p += 1;
            p = eat_digits(bytes, p);
            p = eat_exponent(bytes, p);
            return Some(p);
        }
        // Form 2: digits mandatory-exponent (no dot)
        let after = eat_exponent(bytes, p);
        if after > p {
            return Some(after);
        }
    } else if p < bytes.len() && bytes[p] == b'.' {
        // Form 3: DOT digits optional-exponent
        p += 1;
        let frac_start = p;
        p = eat_digits(bytes, p);
        if p > frac_start {
            p = eat_exponent(bytes, p);
            return Some(p);
        }
    }

    None
}

/// Iterate over all float literals in `s`, yielding their byte ranges.
pub(crate) fn find_numbers(s: &str) -> impl Iterator<Item = NumMatch> + '_ {
    let bytes = s.as_bytes();
    let mut pos: usize = 0;

    std::iter::from_fn(move || {
        while pos < bytes.len() {
            let b = bytes[pos];

            // Quick check: only digits, '-', and '.' can start a float.
            let could_start = b.is_ascii_digit() || b == b'-' || b == b'.';
            if !could_start {
                pos += 1;
                continue;
            }

            // Guard: predecessor must not be [A-Za-z_#].
            if pos > 0 && !is_valid_num_predecessor(bytes[pos - 1]) {
                pos += 1;
                continue;
            }

            if let Some(end) = try_match_number(bytes, pos) {
                let m = NumMatch { start: pos, end };
                pos = end;
                return Some(m);
            }

            pos += 1;
        }
        None
    })
}
