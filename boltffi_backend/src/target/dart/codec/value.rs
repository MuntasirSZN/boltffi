use boltffi_binding::{BinderId, FieldKey, ValueRef, ValueRoot};

use crate::core::{Error, Result};

use super::super::name_style::Name;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueScope {
    Current(String),
    Fields(Vec<(FieldKey, String)>),
}

impl ValueScope {
    pub fn current(expression: impl Into<String>) -> Self {
        Self::Current(expression.into())
    }

    pub fn fields(fields: Vec<(FieldKey, String)>) -> Self {
        Self::Fields(fields)
    }

    pub fn value(&self, value: &ValueRef) -> Result<String> {
        let root = match value.root() {
            ValueRoot::SelfValue => match self {
                Self::Current(expression) => expression.clone(),
                Self::Fields(_) => {
                    return self.field_root(value.path());
                }
            },
            ValueRoot::Named(name) | ValueRoot::Local(name) => match self {
                Self::Current(_) => Name::new(name).lower_camel()?.to_string(),
                Self::Fields(_) => return self.named_root(name, value.path()),
            },
            ValueRoot::Binder(binder) => binder_name(*binder),
            _ => {
                return Err(Error::UnsupportedTarget {
                    target: "dart",
                    shape: "unknown codec value root",
                });
            }
        };
        render_path(root, value.path())
    }

    fn field_root(&self, path: &[FieldKey]) -> Result<String> {
        let Some((field, rest)) = path.split_first() else {
            return Err(Error::UnsupportedTarget {
                target: "dart",
                shape: "whole encoded payload field scope",
            });
        };
        let Self::Fields(fields) = self else {
            unreachable!();
        };
        let expression = fields
            .iter()
            .find_map(|(key, value)| (key == field).then(|| value.clone()))
            .ok_or(Error::UnsupportedTarget {
                target: "dart",
                shape: "unknown encoded payload field",
            })?;
        render_path(expression, rest)
    }

    fn named_root(
        &self,
        name: &boltffi_binding::CanonicalName,
        path: &[FieldKey],
    ) -> Result<String> {
        let Self::Fields(fields) = self else {
            unreachable!();
        };
        let expression = fields
            .iter()
            .find_map(|(key, value)| match key {
                FieldKey::Named(field) if field == name => Some(value.clone()),
                _ => None,
            })
            .ok_or(Error::UnsupportedTarget {
                target: "dart",
                shape: "unknown named encoded payload field",
            })?;
        render_path(expression, path)
    }
}

pub fn binder_name(binder: BinderId) -> String {
    format!("_l$boltffiValue{}", binder.raw())
}

fn render_path(root: String, path: &[FieldKey]) -> Result<String> {
    path.iter().try_fold(root, |expression, field| {
        Ok(match field {
            FieldKey::Named(name) => {
                format!("{expression}.{}", Name::new(name).lower_camel()?)
            }
            FieldKey::Position(position) => format!("{expression}.${}", position + 1),
            _ => {
                return Err(Error::UnsupportedTarget {
                    target: "dart",
                    shape: "unknown codec value field",
                });
            }
        })
    })
}
