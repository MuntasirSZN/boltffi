use crate::bridge::{
    c::TypeFragment,
    jni::{CallbackSuccessOutValue, CallbackSuccessOutWriter},
};

pub struct CallbackSuccessOutWriterView {
    pub symbol: String,
    pub value_jni_type: TypeFragment,
    pub value_c_type: TypeFragment,
    pub writes_scalar: bool,
    pub writes_bytes: bool,
    pub writes_record: bool,
}

impl CallbackSuccessOutWriterView {
    pub fn from_writer(writer: &CallbackSuccessOutWriter) -> Self {
        let value = writer.value();
        Self {
            symbol: writer.symbol().to_string(),
            value_jni_type: value
                .jni_type()
                .map(|ty| ty.as_type_fragment())
                .unwrap_or_else(|| TypeFragment::new("jbyteArray")),
            value_c_type: value
                .c_type()
                .cloned()
                .unwrap_or_else(|| TypeFragment::new("FfiBuf_u8")),
            writes_scalar: matches!(value, CallbackSuccessOutValue::Scalar { .. }),
            writes_bytes: matches!(value, CallbackSuccessOutValue::Bytes),
            writes_record: matches!(value, CallbackSuccessOutValue::Record { .. }),
        }
    }
}
