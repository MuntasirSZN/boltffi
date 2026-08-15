use std::fmt;

use boltffi_binding::DocComment;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Documentation {
    lines: Vec<String>,
}

impl Documentation {
    pub fn new(documentation: Option<&DocComment>) -> Self {
        Self {
            lines: documentation
                .map(DocComment::as_str)
                .map(|text| text.lines().map(Self::sanitize_line).collect())
                .unwrap_or_default(),
        }
    }

    pub fn indented(&self, indentation: &str) -> String {
        if self.lines.is_empty() {
            return String::new();
        }
        let mut block = format!("{indentation}/**\n");
        self.lines.iter().for_each(|line| {
            block.push_str(indentation);
            block.push_str(" *");
            if !line.is_empty() {
                block.push(' ');
                block.push_str(line);
            }
            block.push('\n');
        });
        block.push_str(indentation);
        block.push_str(" */\n");
        block
    }

    fn sanitize_line(line: &str) -> String {
        line.replace("/*", "/ *").replace("*/", "* /")
    }
}

impl fmt::Display for Documentation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.indented(""))
    }
}

#[cfg(test)]
mod tests {
    use boltffi_binding::DocComment;

    use super::Documentation;

    #[test]
    fn renders_nothing_without_doc_text() {
        assert_eq!(Documentation::new(None).to_string(), "");
    }

    #[test]
    fn renders_blank_lines_as_bare_asterisks() {
        let doc_comment = DocComment::new("First paragraph.\n\nSecond paragraph.");

        assert_eq!(
            Documentation::new(Some(&doc_comment)).indented("    "),
            "    /**\n     * First paragraph.\n     *\n     * Second paragraph.\n     */\n"
        );
    }

    #[test]
    fn neutralizes_embedded_block_comment_markers() {
        let doc_comment = DocComment::new("Ends */ safely and /* opens nothing.");

        let rendered = Documentation::new(Some(&doc_comment)).to_string();

        assert_eq!(
            rendered,
            "/**\n * Ends * / safely and / * opens nothing.\n */\n"
        );
    }
}
