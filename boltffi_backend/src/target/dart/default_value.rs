use boltffi_binding::DefaultValue;

use crate::core::Result;

use super::{name_style::Name, syntax::Literal, unsupported};

pub fn literal(value: &DefaultValue) -> Result<Literal> {
    let source = match value {
        DefaultValue::Bool(value) => value.to_string(),
        DefaultValue::Integer(value) => value.get().to_string(),
        DefaultValue::Float(value) => value.to_f64().to_string(),
        DefaultValue::String(value) => format!("{value:?}"),
        DefaultValue::EnumVariant {
            enum_name,
            variant_name,
        } => format!(
            "{}.{}",
            Name::new(enum_name).upper_camel()?,
            Name::new(variant_name).lower_camel()?,
        ),
        DefaultValue::Null => "null".to_owned(),
        _ => return unsupported("unknown default value"),
    };
    Ok(Literal::new(source))
}
