use crate::core::{Error, Result};

use super::{C_BRIDGE_CONTRACT, Identifier, Type};

/// A C function parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Parameter {
    name: Identifier,
    ty: Type,
    role: ParameterRole,
}

/// Position of a C ABI parameter in a function declaration.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct ParameterIndex {
    index: usize,
}

/// Source-level parameter group represented by one or more C ABI parameters.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ParameterGroup {
    /// One source parameter maps to one C ABI parameter.
    Value(ParameterIndex),
    /// One source parameter maps to a borrowed byte pointer and byte length.
    ByteSlice(ByteSliceParameter),
    /// One closure parameter maps to call, context, and release C ABI parameters.
    Closure(ClosureParameter),
}

/// C ABI parameters that carry one borrowed byte slice argument.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ByteSliceParameter {
    name: Identifier,
    pointer: ParameterIndex,
    length: ParameterIndex,
}

/// C ABI parameters that carry one closure argument.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ClosureParameter {
    name: Identifier,
    call: ParameterIndex,
    context: ParameterIndex,
    release: ParameterIndex,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ParameterRole {
    Value,
    BytePointer(Identifier),
    ByteLength(Identifier),
    ClosureCall(Identifier),
    ClosureContext(Identifier),
    ClosureRelease(Identifier),
}

impl Parameter {
    /// Returns the parameter name.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the parameter type.
    pub fn ty(&self) -> &Type {
        &self.ty
    }
}

impl Parameter {
    /// Creates a value C ABI parameter.
    pub fn new(name: impl Into<String>, ty: Type) -> Result<Self> {
        Self::with_role(name, ty, ParameterRole::Value)
    }

    /// Creates the pointer half of a borrowed byte-slice C ABI parameter group.
    pub fn byte_pointer(name: &str) -> Result<Self> {
        Self::with_role(
            format!("{name}_ptr"),
            Type::ConstPointer(Box::new(Type::Uint8)),
            ParameterRole::BytePointer(Identifier::escape(name)?),
        )
    }

    /// Creates the length half of a borrowed byte-slice C ABI parameter group.
    pub fn byte_length(name: &str) -> Result<Self> {
        Self::with_role(
            format!("{name}_len"),
            Type::PointerWidth,
            ParameterRole::ByteLength(Identifier::escape(name)?),
        )
    }

    /// Creates the call function pointer in a closure C ABI parameter group.
    pub fn closure_call(name: &str, ty: Type) -> Result<Self> {
        Self::with_role(
            format!("{name}_call"),
            ty,
            ParameterRole::ClosureCall(Identifier::escape(name)?),
        )
    }

    /// Creates the context pointer in a closure C ABI parameter group.
    pub fn closure_context(name: &str) -> Result<Self> {
        Self::with_role(
            format!("{name}_context"),
            Type::MutPointer(Box::new(Type::Void)),
            ParameterRole::ClosureContext(Identifier::escape(name)?),
        )
    }

    /// Creates the release function pointer in a closure C ABI parameter group.
    pub fn closure_release(name: &str) -> Result<Self> {
        Self::with_role(
            format!("{name}_release"),
            Type::FunctionPointer {
                returns: Box::new(Type::Void),
                params: vec![Type::MutPointer(Box::new(Type::Void))],
            },
            ParameterRole::ClosureRelease(Identifier::escape(name)?),
        )
    }

    fn with_role(name: impl Into<String>, ty: Type, role: ParameterRole) -> Result<Self> {
        Ok(Self {
            name: Identifier::escape(name)?,
            ty,
            role,
        })
    }
}

impl ParameterIndex {
    /// Returns the zero-based C ABI parameter position.
    pub const fn position(self) -> usize {
        self.index
    }

    const fn new(index: usize) -> Self {
        Self { index }
    }
}

impl ParameterGroup {
    /// Builds source-level parameter groups from flat C ABI parameters.
    pub fn from_params(params: &[Parameter]) -> Result<Vec<Self>> {
        let mut index = 0;
        std::iter::from_fn(|| {
            (index < params.len()).then(|| {
                let group = Self::from_param(params, index);
                index += group.as_ref().map_or(1, Self::width);
                group
            })
        })
        .collect()
    }

    fn from_param(params: &[Parameter], index: usize) -> Result<Self> {
        match &params[index].role {
            ParameterRole::Value => Ok(Self::Value(ParameterIndex::new(index))),
            ParameterRole::BytePointer(name) => {
                ByteSliceParameter::from_params(params, index, name).map(Self::ByteSlice)
            }
            ParameterRole::ByteLength(_) => Err(Error::BrokenBridgeContract {
                bridge: C_BRIDGE_CONTRACT,
                invariant: "byte slice parameter group does not start with pointer parameter",
            }),
            ParameterRole::ClosureCall(name) => {
                ClosureParameter::from_params(params, index, name).map(Self::Closure)
            }
            ParameterRole::ClosureContext(_) | ParameterRole::ClosureRelease(_) => {
                Err(Error::BrokenBridgeContract {
                    bridge: C_BRIDGE_CONTRACT,
                    invariant: "closure parameter group does not start with call parameter",
                })
            }
        }
    }

    fn width(&self) -> usize {
        match self {
            Self::Value(_) => 1,
            Self::ByteSlice(_) => 2,
            Self::Closure(_) => 3,
        }
    }
}

impl ByteSliceParameter {
    /// Returns the source parameter name.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the byte pointer parameter position.
    pub const fn pointer(&self) -> ParameterIndex {
        self.pointer
    }

    /// Returns the byte length parameter position.
    pub const fn length(&self) -> ParameterIndex {
        self.length
    }
}

impl ByteSliceParameter {
    fn from_params(params: &[Parameter], pointer: usize, name: &Identifier) -> Result<Self> {
        let length = pointer + 1;
        let length_role = params.get(length).map(|parameter| &parameter.role).ok_or(
            Error::BrokenBridgeContract {
                bridge: C_BRIDGE_CONTRACT,
                invariant: "byte slice parameter group is missing length parameter",
            },
        )?;

        if !length_role.is_byte_length(name) {
            return Err(Error::BrokenBridgeContract {
                bridge: C_BRIDGE_CONTRACT,
                invariant: "byte slice parameter group has mismatched length parameter",
            });
        }

        Ok(Self {
            name: name.clone(),
            pointer: ParameterIndex::new(pointer),
            length: ParameterIndex::new(length),
        })
    }
}

impl ClosureParameter {
    /// Returns the source parameter name.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the call function pointer parameter position.
    pub const fn call(&self) -> ParameterIndex {
        self.call
    }

    /// Returns the callback context parameter position.
    pub const fn context(&self) -> ParameterIndex {
        self.context
    }

    /// Returns the callback release function parameter position.
    pub const fn release(&self) -> ParameterIndex {
        self.release
    }
}

impl ClosureParameter {
    fn from_params(params: &[Parameter], call: usize, name: &Identifier) -> Result<Self> {
        let context = call + 1;
        let release = call + 2;
        let context_role = params.get(context).map(|parameter| &parameter.role).ok_or(
            Error::BrokenBridgeContract {
                bridge: C_BRIDGE_CONTRACT,
                invariant: "closure parameter group is missing context parameter",
            },
        )?;
        let release_role = params.get(release).map(|parameter| &parameter.role).ok_or(
            Error::BrokenBridgeContract {
                bridge: C_BRIDGE_CONTRACT,
                invariant: "closure parameter group is missing release parameter",
            },
        )?;

        if !context_role.is_closure_context(name) || !release_role.is_closure_release(name) {
            return Err(Error::BrokenBridgeContract {
                bridge: C_BRIDGE_CONTRACT,
                invariant: "closure parameter group has mismatched context or release parameter",
            });
        }

        Ok(Self {
            name: name.clone(),
            call: ParameterIndex::new(call),
            context: ParameterIndex::new(context),
            release: ParameterIndex::new(release),
        })
    }
}

impl ParameterRole {
    fn is_byte_length(&self, expected: &Identifier) -> bool {
        matches!(self, Self::ByteLength(name) if name == expected)
    }

    fn is_closure_context(&self, expected: &Identifier) -> bool {
        matches!(self, Self::ClosureContext(name) if name == expected)
    }

    fn is_closure_release(&self, expected: &Identifier) -> bool {
        matches!(self, Self::ClosureRelease(name) if name == expected)
    }
}
