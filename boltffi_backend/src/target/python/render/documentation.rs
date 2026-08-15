use boltffi_binding::DocComment;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Documentation(Option<String>);

impl Documentation {
    pub fn new(documentation: Option<&DocComment>) -> Self {
        Self(
            documentation
                .map(DocComment::as_str)
                .map(str::trim_end)
                .filter(|text| !text.trim().is_empty())
                .map(Self::escape),
        )
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }

    pub fn docstring(&self, indentation: &str) -> String {
        match self.render(indentation) {
            Some(block) => format!("\n{indentation}{block}"),
            None => String::new(),
        }
    }

    pub fn literal(&self) -> String {
        self.render("").unwrap_or_default()
    }

    fn render(&self, indentation: &str) -> Option<String> {
        let text = self.0.as_deref()?;
        let mut lines = text.lines();
        let first = lines.next()?;
        let Some(second) = lines.next() else {
            return Some(format!("\"\"\"{first}\"\"\""));
        };
        let mut block = format!("\"\"\"{first}");
        std::iter::once(second).chain(lines).for_each(|line| {
            block.push('\n');
            if !line.is_empty() {
                block.push_str(indentation);
                block.push_str(line);
            }
        });
        block.push('\n');
        block.push_str(indentation);
        block.push_str("\"\"\"");
        Some(block)
    }

    fn escape(text: &str) -> String {
        text.char_indices().fold(
            String::with_capacity(text.len()),
            |mut escaped, (index, character)| {
                match character {
                    '\\' => escaped.push_str("\\\\"),
                    '"' => {
                        if text
                            .as_bytes()
                            .get(index + 1)
                            .is_none_or(|following| *following == b'"')
                        {
                            escaped.push('\\');
                        }
                        escaped.push('"');
                    }
                    character => escaped.push(character),
                }
                escaped
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use boltffi_binding::DocComment;

    use super::Documentation;

    #[test]
    fn absent_documentation_renders_nothing() {
        let documentation = Documentation::new(None);

        assert!(documentation.is_empty());
        assert_eq!(documentation.docstring("    "), "");
    }

    #[test]
    fn single_line_documentation_renders_on_one_line() {
        let doc_comment = DocComment::new("Returns the stored value.");

        assert_eq!(
            Documentation::new(Some(&doc_comment)).docstring("    "),
            "\n    \"\"\"Returns the stored value.\"\"\""
        );
    }

    #[test]
    fn multi_line_documentation_keeps_blank_lines_unindented() {
        let doc_comment = DocComment::new("A point.\n\nCarries two coordinates.");

        assert_eq!(
            Documentation::new(Some(&doc_comment)).docstring("    "),
            "\n    \"\"\"A point.\n\n    Carries two coordinates.\n    \"\"\""
        );
    }

    #[test]
    fn literal_documents_native_classes_through_an_assignment() {
        let doc_comment = DocComment::new("A point.\n\nCarries two coordinates.");

        assert_eq!(
            Documentation::new(Some(&doc_comment)).literal(),
            "\"\"\"A point.\n\nCarries two coordinates.\n\"\"\""
        );
    }

    #[test]
    fn embedded_triple_quotes_cannot_close_the_docstring() {
        let doc_comment = DocComment::new("Use \"\"\"text\"\"\" for docstrings.");

        assert_eq!(
            Documentation::new(Some(&doc_comment)).docstring(""),
            "\n\"\"\"Use \\\"\\\"\"text\\\"\\\"\" for docstrings.\"\"\""
        );
    }

    #[test]
    fn trailing_backslash_cannot_escape_the_closing_delimiter() {
        let doc_comment = DocComment::new("Matches \\d+\\");

        assert_eq!(
            Documentation::new(Some(&doc_comment)).docstring(""),
            "\n\"\"\"Matches \\\\d+\\\\\"\"\""
        );
    }
}
