use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::LineColumn;
use syn::visit::Visit;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum DeclarationKind {
    Record,
    Enumeration,
}

pub struct Declaration {
    name: String,
    kind: DeclarationKind,
    source: PathBuf,
    offset: usize,
    local_scope: Option<syn::File>,
}

#[derive(Clone)]
enum Scope {
    Module,
    Block(Vec<syn::Item>),
}

struct ScopeFinder<'target> {
    name: &'target str,
    kind: DeclarationKind,
    target_ordinal: usize,
    observed: usize,
    current: Scope,
    scope: Option<Scope>,
}

impl Declaration {
    pub fn from_macro_input(item: &proc_macro::TokenStream) -> syn::Result<Self> {
        let parsed = syn::parse::<syn::Item>(item.clone())?;
        let (name, kind, invocation) = match parsed {
            syn::Item::Struct(item) => (
                item.ident.to_string(),
                DeclarationKind::Record,
                item.ident.span().unwrap(),
            ),
            syn::Item::Enum(item) => (
                item.ident.to_string(),
                DeclarationKind::Enumeration,
                item.ident.span().unwrap(),
            ),
            item => {
                return Err(syn::Error::new_spanned(
                    item,
                    "data runtime requires a struct or enum declaration",
                ));
            }
        };
        let source = invocation.local_file().ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "data source file is unavailable",
            )
        })?;
        let location = LineColumn {
            line: invocation.line(),
            column: invocation.column(),
        };
        let source_text = fs::read_to_string(&source).map_err(|error| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("read data source `{}`: {error}", source.display()),
            )
        })?;
        let offset = Self::source_offset(&source_text, location).ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "locate data declaration `{name}` at {}:{} in `{}`",
                    location.line,
                    location.column,
                    source.display()
                ),
            )
        })?;
        let target_ordinal = Self::declaration_ordinal(&source_text, &name, kind, offset)
            .ok_or_else(|| {
                syn::Error::new(
                    proc_macro2::Span::call_site(),
                    format!(
                        "locate data declaration `{name}` at {}:{} in `{}`",
                        location.line,
                        location.column,
                        source.display()
                    ),
                )
            })?;
        let syntax = syn::parse_file(&source_text)?;
        let scope = ScopeFinder::find(&syntax, &name, kind, target_ordinal).ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "locate data declaration `{name}` at {}:{} in `{}`",
                    location.line,
                    location.column,
                    source.display()
                ),
            )
        })?;
        let local_scope = match scope {
            Scope::Module => None,
            Scope::Block(items) => Some(syn::File {
                shebang: None,
                attrs: Vec::new(),
                items,
            }),
        };
        Ok(Self {
            name,
            kind,
            source,
            offset,
            local_scope,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> DeclarationKind {
        self.kind
    }

    pub fn source(&self) -> &Path {
        &self.source
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn local_scope(&self) -> Option<&syn::File> {
        self.local_scope.as_ref()
    }

    fn source_offset(source: &str, location: LineColumn) -> Option<usize> {
        let line_start = std::iter::once(0)
            .chain(
                source
                    .bytes()
                    .enumerate()
                    .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
            )
            .nth(location.line.checked_sub(1)?)?;
        Some(line_start + location.column)
    }

    fn declaration_ordinal(
        source: &str,
        name: &str,
        kind: DeclarationKind,
        target_offset: usize,
    ) -> Option<usize> {
        let keyword = match kind {
            DeclarationKind::Record => "struct",
            DeclarationKind::Enumeration => "enum",
        };
        let expected_name = name.strip_prefix("r#").unwrap_or(name);
        let mut expects_name = false;
        let mut ordinal = 0;

        rustc_lexer::tokenize(source)
            .scan(0, |offset, token| {
                let start = *offset;
                *offset += token.len;
                Some((token.kind, start, &source[start..*offset]))
            })
            .find_map(|(token, offset, spelling)| match token {
                rustc_lexer::TokenKind::Whitespace
                | rustc_lexer::TokenKind::LineComment
                | rustc_lexer::TokenKind::BlockComment { .. } => None,
                rustc_lexer::TokenKind::Ident if spelling == keyword => {
                    expects_name = true;
                    None
                }
                rustc_lexer::TokenKind::Ident | rustc_lexer::TokenKind::RawIdent
                    if expects_name =>
                {
                    expects_name = false;
                    let declaration_name = spelling.strip_prefix("r#").unwrap_or(spelling);
                    if declaration_name != expected_name {
                        return None;
                    }
                    let current = ordinal;
                    ordinal += 1;
                    (offset <= target_offset && target_offset < offset + spelling.len())
                        .then_some(current)
                }
                _ => {
                    expects_name = false;
                    None
                }
            })
    }
}

impl<'target> ScopeFinder<'target> {
    fn find(
        syntax: &syn::File,
        name: &'target str,
        kind: DeclarationKind,
        target_ordinal: usize,
    ) -> Option<Scope> {
        let mut finder = Self {
            name,
            kind,
            target_ordinal,
            observed: 0,
            current: Scope::Module,
            scope: None,
        };
        finder.visit_file(syntax);
        finder.scope
    }

    fn observe(&mut self, kind: DeclarationKind, name: &syn::Ident) {
        if self.scope.is_some() || self.kind != kind || name != self.name {
            return;
        }
        let ordinal = self.observed;
        self.observed += 1;
        if ordinal == self.target_ordinal {
            self.scope = Some(self.current.clone());
        }
    }
}

impl<'syntax> Visit<'syntax> for ScopeFinder<'_> {
    fn visit_file(&mut self, syntax: &'syntax syn::File) {
        self.current = Scope::Module;
        syntax.items.iter().for_each(|item| self.visit_item(item));
    }

    fn visit_item_mod(&mut self, module: &'syntax syn::ItemMod) {
        let Some((_, items)) = &module.content else {
            return;
        };
        let enclosing = std::mem::replace(&mut self.current, Scope::Module);
        items.iter().for_each(|item| self.visit_item(item));
        self.current = enclosing;
    }

    fn visit_block(&mut self, block: &'syntax syn::Block) {
        if self.scope.is_some() {
            return;
        }
        let items = block
            .stmts
            .iter()
            .filter_map(|statement| match statement {
                syn::Stmt::Item(item) => Some(item.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let enclosing = std::mem::replace(&mut self.current, Scope::Block(items));
        block
            .stmts
            .iter()
            .for_each(|statement| self.visit_stmt(statement));
        self.current = enclosing;
    }

    fn visit_item_struct(&mut self, item: &'syntax syn::ItemStruct) {
        self.observe(DeclarationKind::Record, &item.ident);
    }

    fn visit_item_enum(&mut self, item: &'syntax syn::ItemEnum) {
        self.observe(DeclarationKind::Enumeration, &item.ident);
    }
}

#[cfg(test)]
mod tests {
    use super::{DeclarationKind, Scope, ScopeFinder};

    #[test]
    fn finds_data_declarations_in_their_function_block() {
        let syntax = syn::parse_file(
            "fn roundtrip() {\n#[data]\nstruct Point { x: f64 }\n#[data]\nstruct Pair { point: Point }\n}\n",
        )
        .expect("source parses");
        let scope =
            ScopeFinder::find(&syntax, "Pair", DeclarationKind::Record, 0).expect("scope exists");

        assert!(matches!(scope, Scope::Block(items) if items.len() == 2));
    }

    #[test]
    fn distinguishes_module_data_from_local_data() {
        let syntax = syn::parse_file("#[data]\nstruct Point { x: f64 }\n").expect("source parses");
        let scope =
            ScopeFinder::find(&syntax, "Point", DeclarationKind::Record, 0).expect("scope exists");

        assert!(matches!(scope, Scope::Module));
    }

    #[test]
    fn distinguishes_same_named_declarations_by_source_order() {
        let syntax = syn::parse_file(
            "fn first() { struct Point; }\nfn second() { struct Point; struct Pair(Point); }\n",
        )
        .expect("source parses");
        let scope =
            ScopeFinder::find(&syntax, "Point", DeclarationKind::Record, 1).expect("scope exists");

        assert!(matches!(scope, Scope::Block(items) if items.len() == 2));
    }
}
