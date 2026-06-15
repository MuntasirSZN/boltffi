use std::path::{Path, PathBuf};

use crate::core::{Error, Result};

/// Path of one generated backend output file.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct FilePath(PathBuf);

impl FilePath {
    /// Creates a generated file path.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if path.as_os_str().is_empty() {
            Err(Error::EmptyFilePath)
        } else {
            Ok(Self(path))
        }
    }

    /// Returns the path as a standard path value.
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// One text fragment assigned to a generated file.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Fragment {
    path: FilePath,
    text: String,
}

impl Fragment {
    /// Creates a text fragment for a file.
    pub fn new(path: FilePath, text: impl Into<String>) -> Self {
        Self {
            path,
            text: text.into(),
        }
    }

    /// Returns the target file path.
    pub const fn path(&self) -> &FilePath {
        &self.path
    }

    /// Returns the fragment text.
    pub fn text(&self) -> &str {
        &self.text
    }

    fn into_parts(self) -> (FilePath, String) {
        (self.path, self.text)
    }
}

/// Diagnostic emitted while rendering generated output.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Diagnostic {
    message: String,
}

impl Diagnostic {
    /// Creates a diagnostic message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the diagnostic message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Rendered backend output before final file assembly.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct Emitted {
    fragments: Vec<Fragment>,
    diagnostics: Vec<Diagnostic>,
}

impl Emitted {
    /// Creates empty rendered output.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates rendered output from one fragment.
    pub fn fragment(fragment: Fragment) -> Self {
        Self {
            fragments: vec![fragment],
            diagnostics: Vec::new(),
        }
    }

    /// Creates rendered output from fragments.
    pub fn fragments(fragments: impl IntoIterator<Item = Fragment>) -> Self {
        Self {
            fragments: fragments.into_iter().collect(),
            diagnostics: Vec::new(),
        }
    }

    /// Creates rendered output from one diagnostic.
    pub fn diagnostic(diagnostic: Diagnostic) -> Self {
        Self {
            fragments: Vec::new(),
            diagnostics: vec![diagnostic],
        }
    }

    /// Returns generated fragments.
    pub fn file_fragments(&self) -> &[Fragment] {
        &self.fragments
    }

    /// Returns diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Appends another rendered output value.
    pub fn append(&mut self, other: Self) {
        self.fragments.extend(other.fragments);
        self.diagnostics.extend(other.diagnostics);
    }

    /// Combines multiple rendered output values.
    pub fn combine(outputs: impl IntoIterator<Item = Self>) -> Self {
        outputs
            .into_iter()
            .fold(Self::empty(), |mut combined, output| {
                combined.append(output);
                combined
            })
    }

    /// Splits this value into fragments and diagnostics.
    pub fn into_parts(self) -> (Vec<Fragment>, Vec<Diagnostic>) {
        (self.fragments, self.diagnostics)
    }
}

/// Layout policy for generated backend files.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct FileLayout;

impl FileLayout {
    /// Creates a file layout.
    pub const fn new() -> Self {
        Self
    }

    /// Assembles emitted fragments into generated output.
    pub fn assemble(self, emitted: Emitted) -> GeneratedOutput {
        let (fragments, diagnostics) = emitted.into_parts();
        GeneratedOutput::new(GeneratedFile::assemble(fragments), diagnostics)
    }
}

/// Generated backend output assembled from bridge and host render fragments.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct GeneratedOutput {
    files: Vec<GeneratedFile>,
    diagnostics: Vec<Diagnostic>,
}

impl GeneratedOutput {
    /// Creates generated backend output.
    pub fn new(files: Vec<GeneratedFile>, diagnostics: Vec<Diagnostic>) -> Self {
        Self { files, diagnostics }
    }

    /// Returns generated files.
    pub fn files(&self) -> &[GeneratedFile] {
        &self.files
    }

    /// Returns non-fatal render diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Splits this output into generated files and diagnostics.
    pub fn into_parts(self) -> (Vec<GeneratedFile>, Vec<Diagnostic>) {
        (self.files, self.diagnostics)
    }
}

/// One complete generated backend file.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct GeneratedFile {
    path: FilePath,
    contents: String,
}

impl GeneratedFile {
    /// Creates a generated file.
    pub fn new(path: FilePath, contents: impl Into<String>) -> Self {
        Self {
            path,
            contents: contents.into(),
        }
    }

    /// Returns the generated file path.
    pub const fn path(&self) -> &FilePath {
        &self.path
    }

    /// Returns the generated file contents.
    pub fn contents(&self) -> &str {
        &self.contents
    }

    fn assemble(fragments: impl IntoIterator<Item = Fragment>) -> Vec<Self> {
        fragments
            .into_iter()
            .fold(Vec::new(), |mut files, fragment| {
                let (path, text) = fragment.into_parts();
                if let Some(file) = files.iter_mut().find(|file| file.path == path) {
                    file.contents.push_str(&text);
                } else {
                    files.push(Self::new(path, text));
                }
                files
            })
    }
}
