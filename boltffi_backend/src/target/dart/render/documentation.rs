use std::fmt;

use boltffi_binding::DocComment;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Documentation {
    source: String,
}

impl Documentation {
    pub fn new(doc: Option<&DocComment>, indentation: usize) -> Self {
        let prefix = " ".repeat(indentation);
        let source = doc
            .map(|documentation| {
                documentation
                    .as_str()
                    .lines()
                    .map(|line| match line.is_empty() {
                        true => format!("{prefix}///\n"),
                        false => format!("{prefix}/// {line}\n"),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self { source }
    }
}

impl fmt::Display for Documentation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.source)
    }
}
