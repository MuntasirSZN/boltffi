use std::fmt;

/// A source shape the scanner cannot turn into an AST node yet.
///
/// Each variant names a concrete source shape that was rejected rather
/// than silently dropped, so a caller can report it against the original
/// Rust the user wrote.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScanError {
    /// A type expression the scanner does not recognize at this stage,
    /// carrying the source spelling for diagnostics.
    UnsupportedType {
        /// The source type as written.
        spelling: String,
    },
    /// A parameter whose pattern is not a plain name binding.
    UnnamedParameter,
    /// A receiver (`self`) appeared on a free function.
    ReceiverOnFreeFunction,
    /// A struct without named fields (tuple or unit) appeared where a
    /// record is expected.
    TupleOrUnitStruct,
}

impl ScanError {
    pub(crate) fn unsupported_type(ty: &syn::Type) -> Self {
        Self::UnsupportedType {
            spelling: type_spelling(ty),
        }
    }
}

fn type_spelling(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => type_path
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        syn::Type::Reference(reference) => format!("&{}", type_spelling(&reference.elem)),
        _ => "unrecognized type".to_owned(),
    }
}

impl fmt::Display for ScanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedType { spelling } => {
                write!(formatter, "unsupported source type `{spelling}`")
            }
            Self::UnnamedParameter => formatter.write_str("parameter pattern is not a plain name"),
            Self::ReceiverOnFreeFunction => {
                formatter.write_str("free function cannot have a receiver")
            }
            Self::TupleOrUnitStruct => {
                formatter.write_str("tuple and unit structs are not supported as records yet")
            }
        }
    }
}

impl std::error::Error for ScanError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn ty(source: &str) -> syn::Type {
        syn::parse_str(source).expect("valid type")
    }

    #[test]
    fn unsupported_path_type_preserves_qualified_spelling() {
        assert_eq!(
            ScanError::unsupported_type(&ty("crate::domain::Point")),
            ScanError::UnsupportedType {
                spelling: "crate::domain::Point".to_owned()
            }
        );
    }

    #[test]
    fn unsupported_reference_type_preserves_reference_shape() {
        assert_eq!(
            ScanError::unsupported_type(&ty("&Point")),
            ScanError::UnsupportedType {
                spelling: "&Point".to_owned()
            }
        );
    }

    #[test]
    fn display_messages_are_stable_and_specific() {
        assert_eq!(
            ScanError::UnsupportedType {
                spelling: "Point".to_owned()
            }
            .to_string(),
            "unsupported source type `Point`"
        );
        assert_eq!(
            ScanError::UnnamedParameter.to_string(),
            "parameter pattern is not a plain name"
        );
        assert_eq!(
            ScanError::ReceiverOnFreeFunction.to_string(),
            "free function cannot have a receiver"
        );
        assert_eq!(
            ScanError::TupleOrUnitStruct.to_string(),
            "tuple and unit structs are not supported as records yet"
        );
    }
}
