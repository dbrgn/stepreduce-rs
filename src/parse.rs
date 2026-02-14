use std::io::BufRead;

/// The three sections of a STEP file: everything before `DATA;`, the data
/// entity lines, and everything from `ENDSEC;` onward.
pub(crate) struct ParseResult {
    pub header: Vec<String>,
    pub data: Vec<String>,
    pub footer: Vec<String>,
}

/// Parse a STEP file from a buffered reader into its header, data, and footer
/// sections.
///
/// Multi-line data entities (lines not ending with `;`) are joined into a
/// single string. The header and footer lines are preserved verbatim (with
/// trailing whitespace trimmed from header lines).
pub(crate) fn parse_data_section(reader: impl BufRead) -> ParseResult {
    let mut result = ParseResult {
        header: Vec::new(),
        data: Vec::new(),
        footer: Vec::new(),
    };

    let mut past_header = false;
    let mut past_data = false;
    let mut continuing = false;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if past_header {
            if past_data || line.contains("ENDSEC;") {
                past_data = true;
                result.footer.push(line);
            } else {
                let trimmed = line.trim().to_string();

                if continuing {
                    if trimmed
                        .as_bytes()
                        .first()
                        .is_some_and(|b| b.is_ascii_alphabetic())
                    {
                        result.data.last_mut().unwrap().push(' ');
                    }
                    result.data.last_mut().unwrap().push_str(&trimmed);
                } else {
                    result.data.push(trimmed);
                }

                continuing = !line.trim_end().ends_with(';');
            }
        } else {
            if line.contains("DATA;") {
                past_header = true;
            }
            result.header.push(line.trim_end().to_string());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn basic_parse() {
        let input = "\
HEADER;
FILE_DESCRIPTION(('test'),'2;1');
ENDSEC;
DATA;
#1=PRODUCT('widget','widget',$,(#2));
#2=PRODUCT_CONTEXT('',#3,'design');
ENDSEC;
END-ISO-10303-21;
";
        let reader = Cursor::new(input);
        let result = parse_data_section(reader);

        assert_eq!(result.header.len(), 4); // HEADER; through DATA;
        assert_eq!(result.data.len(), 2);
        assert_eq!(result.footer.len(), 2); // ENDSEC; END-ISO...
        assert!(result.data[0].starts_with("#1="));
    }

    #[test]
    fn continuation_lines() {
        let input = "\
DATA;
#1=LONG_ENTITY('foo',
#2,#3,
#4);
#5=SHORT('bar');
ENDSEC;
";
        let reader = Cursor::new(input);
        let result = parse_data_section(reader);

        assert_eq!(result.data.len(), 2);
        assert!(result.data[0].contains("#2,#3,"));
        assert!(result.data[0].ends_with(';'));
    }
}
