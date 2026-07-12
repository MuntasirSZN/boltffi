use boltffi_binding::{CallbackDecl, ExecutionDecl, Native};

use crate::{
    core::{Error, Result},
    target::java::JavaHost,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CallbackShape {
    Supported,
    Asynchronous,
    ClosureParameter,
}

impl CallbackShape {
    pub fn classify(declaration: &CallbackDecl<Native>) -> Self {
        declaration
            .protocol()
            .vtable()
            .methods()
            .iter()
            .find_map(|method| {
                let callable = method.callable();
                [
                    (!matches!(callable.execution(), ExecutionDecl::Synchronous(_)))
                        .then_some(Self::Asynchronous),
                    callable
                        .params()
                        .iter()
                        .any(|parameter| parameter.payload().as_value().is_none())
                        .then_some(Self::ClosureParameter),
                ]
                .into_iter()
                .flatten()
                .next()
            })
            .unwrap_or(Self::Supported)
    }

    pub fn require_supported(self) -> Result<()> {
        self.unsupported_reason().map_or(Ok(()), |shape| {
            Err(Error::UnsupportedTarget {
                target: JavaHost::TARGET,
                shape,
            })
        })
    }

    pub const fn unsupported_reason(self) -> Option<&'static str> {
        match self {
            Self::Supported => None,
            Self::Asynchronous => Some("asynchronous callback method"),
            Self::ClosureParameter => Some("callback closure parameter"),
        }
    }
}
