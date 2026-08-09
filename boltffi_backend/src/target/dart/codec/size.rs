use boltffi_binding::{
    BinderId, BuiltinType, CallbackId, ClassId, CodecSize, CustomTypeId, ElementCount, EnumId,
    MapKind, Op, Primitive, RecordId, ValueRef,
};

use crate::core::Result;

use super::{ValueScope, primitive_size, value::binder_name};

pub struct Sizer {
    scope: ValueScope,
}

impl Sizer {
    pub fn new(scope: ValueScope) -> Self {
        Self { scope }
    }

    fn value(&self, value: &ValueRef) -> Result<String> {
        self.scope.value(value)
    }
}

impl CodecSize for Sizer {
    type Expr = Result<String>;

    fn primitive(&mut self, primitive: Primitive, _: &ValueRef) -> Self::Expr {
        Ok(primitive_size(primitive).to_string())
    }

    fn string(&mut self, value: &ValueRef) -> Self::Expr {
        Ok(format!(
            "4 + $$convert.utf8.encode({}).length",
            self.value(value)?
        ))
    }

    fn interned_string(&mut self, _: &[String], value: &ValueRef) -> Self::Expr {
        Ok(format!(
            "1 + 4 + $$convert.utf8.encode({}).length",
            self.value(value)?
        ))
    }

    fn bytes(&mut self, value: &ValueRef) -> Self::Expr {
        Ok(format!("4 + {}.lengthInBytes", self.value(value)?))
    }

    fn direct_record(&mut self, _: RecordId, value: &ValueRef) -> Self::Expr {
        Ok(format!("{}._m$wireEncodedSize()", self.value(value)?))
    }

    fn encoded_record(&mut self, id: RecordId, value: &ValueRef) -> Self::Expr {
        self.direct_record(id, value)
    }

    fn c_style_enum(&mut self, _: EnumId, _: &ValueRef) -> Self::Expr {
        Ok("4".to_owned())
    }

    fn data_enum(&mut self, _: EnumId, value: &ValueRef) -> Self::Expr {
        Ok(format!("{}._m$wireEncodedSize()", self.value(value)?))
    }

    fn class_handle(&mut self, _: ClassId, _: &ValueRef) -> Self::Expr {
        Ok("8".to_owned())
    }

    fn callback_handle(&mut self, _: CallbackId, _: &ValueRef) -> Self::Expr {
        Ok("16".to_owned())
    }

    fn custom<F>(&mut self, _: CustomTypeId, value: &ValueRef, representation: F) -> Self::Expr
    where
        F: FnOnce(&mut Self, &ValueRef) -> Self::Expr,
    {
        representation(self, value)
    }

    fn builtin(&mut self, kind: BuiltinType, value: &ValueRef) -> Self::Expr {
        match kind {
            BuiltinType::Duration | BuiltinType::SystemTime => Ok("12".to_owned()),
            BuiltinType::Uuid => Ok("16".to_owned()),
            BuiltinType::Url => self.string(value),
        }
    }

    fn optional(&mut self, value: &ValueRef, binder: BinderId, inner: Self::Expr) -> Self::Expr {
        Ok(format!(
            "1 + ({} == null ? 0 : (() {{ final {} = {}; return {}; }})())",
            self.value(value)?,
            binder_name(binder),
            self.value(value)?,
            inner?
        ))
    }

    fn sequence(
        &mut self,
        value: &ValueRef,
        _: &Op<ElementCount>,
        binder: BinderId,
        element: Self::Expr,
    ) -> Self::Expr {
        Ok(format!(
            "4 + {}.fold<int>(0, (_l$size, {}) => _l$size + {})",
            self.value(value)?,
            binder_name(binder),
            element?
        ))
    }

    fn tuple(&mut self, _: &ValueRef, elements: Vec<Self::Expr>) -> Self::Expr {
        Ok(elements
            .into_iter()
            .collect::<Result<Vec<_>>>()?
            .join(" + "))
    }

    fn result(
        &mut self,
        value: &ValueRef,
        binder: BinderId,
        ok: Self::Expr,
        err: Self::Expr,
    ) -> Self::Expr {
        Ok(format!(
            "1 + switch ({}) {{ $$BoltResult$Ok(value: final {}) => {}, $$BoltResult$Err(value: final {}) => {} }}",
            self.value(value)?,
            binder_name(binder),
            ok?,
            binder_name(binder),
            err?
        ))
    }

    fn map(
        &mut self,
        _: MapKind,
        value: &ValueRef,
        key_binder: BinderId,
        key: Self::Expr,
        value_binder: BinderId,
        map_value: Self::Expr,
    ) -> Self::Expr {
        Ok(format!(
            "4 + {}.entries.fold<int>(0, (_l$size, _l$entry) {{ final {} = _l$entry.key; final {} = _l$entry.value; return _l$size + {} + {}; }})",
            self.value(value)?,
            binder_name(key_binder),
            binder_name(value_binder),
            key?,
            map_value?,
        ))
    }
}
