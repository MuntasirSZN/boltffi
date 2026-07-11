use std::fmt;

use askama::Template as AskamaTemplate;
use boltffi_binding::NativeSymbol;

use crate::{
    bridge::jni::{
        JniBridgeContract, NativeMethod, NativeParameter, NativeParameterKind, NativeReturn,
    },
    core::{Error, Result},
    target::java::{
        JavaVersion,
        primitive::Primitive,
        render::signature::{Parameter, ReturnType, ValueType},
        syntax::{ArgumentList, Expression, Identifier, TypeIdentifier, TypeName},
    },
    target::jvm::method::{Parameter as JvmParameter, Parameters as JvmParameters, SlotWidth},
};

#[derive(AskamaTemplate)]
#[template(path = "target/java/native_method.java", escape = "none")]
struct MethodTemplate<'method> {
    method: &'method Method,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Method {
    name: Identifier,
    parameters: JvmParameters<Parameter<Carrier>>,
    returns: MethodReturn,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Carrier {
    Primitive(Primitive),
    ByteArray,
    DirectBuffer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MethodReturn {
    Void,
    Value(Carrier),
    CheckedVoid,
}

impl Method {
    pub fn from_symbol(
        symbol: &NativeSymbol,
        bridge: &JniBridgeContract,
        version: JavaVersion,
    ) -> Result<Self> {
        bridge
            .source_method(symbol.id())
            .ok_or(Error::BrokenBridgeContract {
                bridge: "jni",
                invariant: "source symbol has no JNI native method",
            })
            .and_then(|method| Self::from_contract(method, version))
    }

    pub fn from_contract(method: &NativeMethod, version: JavaVersion) -> Result<Self> {
        let name = Identifier::parse_for(method.c_function().name(), version)?;
        let parameters = method
            .parameters()
            .iter()
            .map(|parameter| Self::parameter_from_contract(parameter, version))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        Parameter::validate_unique(name.as_str(), &parameters)?;
        let parameters = JvmParameters::for_static(parameters)?;
        Ok(Self {
            name,
            parameters,
            returns: MethodReturn::from_contract(method.returns())?,
        })
    }

    pub fn validate_return(&self, returns: &ReturnType) -> Result<()> {
        self.returns.validate_carrier(returns)
    }

    pub fn call(
        &self,
        owner: &TypeIdentifier,
        arguments: impl IntoIterator<Item = Expression>,
    ) -> Result<Expression> {
        let arguments = arguments.into_iter().collect::<Vec<_>>();
        if arguments.len() != self.parameters.len() {
            return Err(Error::BrokenBridgeContract {
                bridge: "jni",
                invariant: "Java native call argument arity matches",
            });
        }
        Ok(Expression::static_call(
            TypeName::named(owner.clone()),
            self.name.clone(),
            arguments.into_iter().collect::<ArgumentList>(),
        ))
    }

    pub fn render(&self) -> Result<String> {
        Ok(MethodTemplate { method: self }.render()?)
    }

    fn name(&self) -> &Identifier {
        &self.name
    }

    fn parameters(&self) -> &[Parameter<Carrier>] {
        self.parameters.as_slice()
    }

    fn returns(&self) -> &MethodReturn {
        &self.returns
    }

    fn parameter_from_contract(
        parameter: &NativeParameter,
        version: JavaVersion,
    ) -> Result<Vec<Parameter<Carrier>>> {
        match parameter.kind() {
            NativeParameterKind::Scalar(scalar) => Ok(vec![Parameter::new(
                Identifier::escape_for(parameter.name().as_str(), version)?,
                Carrier::Primitive(Primitive::from(scalar.ty())),
            )]),
            NativeParameterKind::Bytes(bytes) => Ok(vec![
                Parameter::new(
                    Identifier::escape_for(bytes.name().as_str(), version)?,
                    Carrier::DirectBuffer,
                ),
                Parameter::new(
                    Identifier::escape_for(bytes.length().as_str(), version)?,
                    Carrier::Primitive(Primitive::Int),
                ),
            ]),
            NativeParameterKind::Record(_) => Ok(vec![Parameter::new(
                Identifier::escape_for(parameter.name().as_str(), version)?,
                Carrier::DirectBuffer,
            )]),
            _ => Err(Error::UnsupportedTarget {
                target: "java",
                shape: "non-scalar JNI native method parameter",
            }),
        }
    }
}

impl MethodReturn {
    fn from_contract(returns: &NativeReturn) -> Result<Self> {
        match returns {
            NativeReturn::Void => Ok(Self::Void),
            NativeReturn::Status | NativeReturn::EncodedError => Ok(Self::CheckedVoid),
            NativeReturn::Value(scalar) => Ok(Self::Value(Carrier::Primitive(Primitive::from(
                scalar.jni_type(),
            )))),
            NativeReturn::Bytes | NativeReturn::Record(_) | NativeReturn::StatusWriteback(_) => {
                Ok(Self::Value(Carrier::ByteArray))
            }
            NativeReturn::StatusValue(value) => Self::from_contract(value.value()),
            NativeReturn::EncodedErrorValue(value) => Self::from_contract(value.success().value()),
            _ => Err(Error::UnsupportedTarget {
                target: "java",
                shape: "non-scalar JNI native method return",
            }),
        }
    }

    fn validate_carrier(&self, returns: &ReturnType) -> Result<()> {
        match self {
            Self::Void if matches!(returns, ReturnType::Void) => Ok(()),
            Self::Value(native) if matches!(returns, ReturnType::Value(value) if native.matches(value)) => {
                Ok(())
            }
            Self::CheckedVoid if matches!(returns, ReturnType::Void) => Ok(()),
            _ => Err(Error::BrokenBridgeContract {
                bridge: "jni",
                invariant: "Java and JNI function return shapes match",
            }),
        }
    }
}

impl fmt::Display for MethodReturn {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Void | Self::CheckedVoid => formatter.write_str("void"),
            Self::Value(returns) => returns.fmt(formatter),
        }
    }
}

impl Carrier {
    fn matches(self, value: &ValueType) -> bool {
        match (self, value) {
            (Self::Primitive(native), ValueType::Primitive(public)) => native == *public,
            (Self::ByteArray | Self::DirectBuffer, ValueType::Record(_))
            | (Self::ByteArray, ValueType::Reference(_)) => true,
            _ => false,
        }
    }

    fn slot_width(self) -> SlotWidth {
        match self {
            Self::Primitive(primitive) => primitive.slot_width(),
            Self::ByteArray | Self::DirectBuffer => SlotWidth::Single,
        }
    }
}

impl fmt::Display for Carrier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Primitive(primitive) => primitive.fmt(formatter),
            Self::ByteArray => formatter.write_str("byte[]"),
            Self::DirectBuffer => formatter.write_str("java.nio.ByteBuffer"),
        }
    }
}

impl JvmParameter for Parameter<Carrier> {
    fn slot_width(&self) -> SlotWidth {
        self.ty().slot_width()
    }
}

#[cfg(test)]
mod tests {
    use crate::target::java::{
        primitive::Primitive,
        render::signature::{ReturnType, ValueType},
    };

    use super::{Carrier, MethodReturn};

    #[test]
    fn preserves_checked_void_as_a_native_only_return_state() {
        let direct = MethodReturn::Void;
        let checked = MethodReturn::CheckedVoid;
        let scalar = MethodReturn::Value(Carrier::Primitive(Primitive::Int));

        assert_eq!(direct.to_string(), "void");
        assert_eq!(checked.to_string(), "void");
        assert_eq!(scalar.to_string(), "int");
        assert!(direct.validate_carrier(&ReturnType::Void).is_ok());
        assert!(checked.validate_carrier(&ReturnType::Void).is_ok());
        assert!(
            checked
                .validate_carrier(&ReturnType::Value(ValueType::Primitive(Primitive::Int)))
                .is_err()
        );
    }
}
