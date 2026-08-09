use boltffi_binding::{
    BuiltinType, CallbackId, ClassId, CustomTypeId, EnumId, Primitive, RecordId, TypeRef,
    TypeRefRender,
};

use crate::core::Result;

pub struct ValueSemantics(Strategy);

enum Strategy {
    Direct,
    Optional(Box<ValueSemantics>),
    Sequence(Box<ValueSemantics>),
    Tuple(Vec<ValueSemantics>),
    Result {
        ok: Box<ValueSemantics>,
        err: Box<ValueSemantics>,
    },
    Map {
        key: Box<ValueSemantics>,
        value: Box<ValueSemantics>,
    },
}

impl ValueSemantics {
    pub fn direct() -> Self {
        Self(Strategy::Direct)
    }

    pub fn for_type(ty: &TypeRef) -> Result<Self> {
        ty.render_with(&mut Renderer)
    }

    pub fn equality(&self, left: &str, right: &str) -> String {
        match &self.0 {
            Strategy::Direct => format!("{left} == {right}"),
            Strategy::Optional(inner) => format!(
                "_$$BoltUtil.nullableCompare({left}, {right}, (_l$left, _l$right) => {})",
                inner.equality("_l$left", "_l$right")
            ),
            Strategy::Sequence(element) => format!(
                "_$$BoltUtil.listCompare({left}, {right}, (_l$left, _l$right) => {})",
                element.equality("_l$left", "_l$right")
            ),
            Strategy::Tuple(elements) => {
                let equality = elements
                    .iter()
                    .enumerate()
                    .map(|(index, element)| {
                        element.equality(
                            &format!("({left}).${}", index + 1),
                            &format!("({right}).${}", index + 1),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" && ");
                if equality.is_empty() {
                    "true".to_owned()
                } else {
                    equality
                }
            }
            Strategy::Result { ok, err } => format!(
                "_$$BoltUtil.fallibleCompare({left}, {right}, (_l$left, _l$right) => {}, (_l$left, _l$right) => {})",
                ok.equality("_l$left", "_l$right"),
                err.equality("_l$left", "_l$right")
            ),
            Strategy::Map { key, value } => format!(
                "_$$BoltUtil.mapCompare({left}, {right}, (_l$left, _l$right) => {}, (_l$left, _l$right) => {})",
                key.equality("_l$left", "_l$right"),
                value.equality("_l$left", "_l$right")
            ),
        }
    }

    pub fn hash(&self, value: &str) -> String {
        match &self.0 {
            Strategy::Direct => format!("{value}.hashCode"),
            Strategy::Optional(inner) => format!(
                "({value} == null ? null.hashCode : {})",
                inner.hash(&format!("{value}!"))
            ),
            Strategy::Sequence(element) => format!(
                "_$$BoltUtil.listHash({value}, (_l$value) => {})",
                element.hash("_l$value")
            ),
            Strategy::Tuple(elements) => format!(
                "Object.hashAll([{}])",
                elements
                    .iter()
                    .enumerate()
                    .map(|(index, element)| { element.hash(&format!("({value}).${}", index + 1)) })
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Strategy::Result { ok, err } => format!(
                "switch ({value}) {{ $$BoltResult$Ok(value: final _l$value) => {}, $$BoltResult$Err(value: final _l$value) => {} }}",
                ok.hash("_l$value"),
                err.hash("_l$value")
            ),
            Strategy::Map {
                key,
                value: map_value,
            } => format!(
                "_$$BoltUtil.mapHash({value}, (_l$value) => {}, (_l$value) => {})",
                key.hash("_l$value"),
                map_value.hash("_l$value")
            ),
        }
    }
}

struct Renderer;

impl TypeRefRender for Renderer {
    type Output = Result<ValueSemantics>;

    fn primitive(&mut self, _: Primitive) -> Self::Output {
        Ok(ValueSemantics::direct())
    }

    fn string(&mut self) -> Self::Output {
        Ok(ValueSemantics::direct())
    }

    fn interned_string(&mut self, _: &[String]) -> Self::Output {
        unreachable!("InternedString semantics reached Dart renderer without host capability")
    }

    fn bytes(&mut self) -> Self::Output {
        Ok(ValueSemantics(Strategy::Sequence(Box::new(
            ValueSemantics::direct(),
        ))))
    }

    fn record(&mut self, _: RecordId) -> Self::Output {
        Ok(ValueSemantics::direct())
    }

    fn enumeration(&mut self, _: EnumId) -> Self::Output {
        Ok(ValueSemantics::direct())
    }

    fn class(&mut self, _: ClassId) -> Self::Output {
        Ok(ValueSemantics::direct())
    }

    fn callback(&mut self, _: CallbackId) -> Self::Output {
        Ok(ValueSemantics::direct())
    }

    fn custom(&mut self, _: CustomTypeId) -> Self::Output {
        Ok(ValueSemantics::direct())
    }

    fn builtin(&mut self, _: BuiltinType) -> Self::Output {
        Ok(ValueSemantics::direct())
    }

    fn optional(&mut self, inner: Self::Output) -> Self::Output {
        inner.map(|inner| ValueSemantics(Strategy::Optional(Box::new(inner))))
    }

    fn sequence(&mut self, element: Self::Output) -> Self::Output {
        element.map(|element| ValueSemantics(Strategy::Sequence(Box::new(element))))
    }

    fn tuple(&mut self, elements: Vec<Self::Output>) -> Self::Output {
        elements
            .into_iter()
            .collect::<Result<Vec<_>>>()
            .map(|elements| ValueSemantics(Strategy::Tuple(elements)))
    }

    fn result(&mut self, ok: Self::Output, err: Self::Output) -> Self::Output {
        Ok(ValueSemantics(Strategy::Result {
            ok: Box::new(ok?),
            err: Box::new(err?),
        }))
    }

    fn map(&mut self, key: Self::Output, value: Self::Output) -> Self::Output {
        Ok(ValueSemantics(Strategy::Map {
            key: Box::new(key?),
            value: Box::new(value?),
        }))
    }
}
