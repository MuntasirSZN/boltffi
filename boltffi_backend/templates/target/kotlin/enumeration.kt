{%- if enumeration.c_style() %}
{%- if let Some(value_type) = enumeration.value_type() %}
enum class {{ enumeration.name() }}(val value: {{ value_type }}) {
{%- for variant in enumeration.c_style_variants() %}
    {{ variant.name() }}({{ variant.value() }}){% if !loop.last %},{% else %};{% endif %}
{%- endfor %}

    companion object {
        fun fromValue(value: {{ value_type }}): {{ enumeration.name() }} =
            entries.first { it.value == value }
    }
}
{%- endif %}
{%- else if enumeration.data() %}
sealed class {{ enumeration.name() }} {
    internal abstract fun wireSize(): Int

    internal abstract fun writeTo(writer: WireWriter)

    internal fun toByteArray(): ByteArray {
        val buffer = WireWriterPool.acquire(wireSize())
        val writer = buffer.writer
        try {
            writeTo(writer)
            return buffer.bytes()
        } finally {
            buffer.close()
        }
    }
{% for variant in enumeration.data_variants() %}
{%- if variant.unit() %}
    object {{ variant.name() }} : {{ enumeration.name() }}() {
        internal override fun wireSize(): Int {
            return {{ variant.size() }}
        }

        internal override fun writeTo(writer: WireWriter) {
            {{ variant.tag_write() }}
        }
    }
{%- else %}
    data class {{ variant.name() }}(
{%- for field in variant.fields() %}
        val {{ field.name() }}: {{ field.ty() }}{% if !loop.last %},{% endif %}
{%- endfor %}
    ) : {{ enumeration.name() }}() {
        internal override fun wireSize(): Int {
            return {{ variant.size() }}
        }

        internal override fun writeTo(writer: WireWriter) {
            {{ variant.tag_write() }}
{%- for field in variant.fields() %}
            {{ field.write() }}
{%- endfor %}
        }
    }
{%- endif %}
{%- endfor %}

    companion object {
        internal fun fromReader(reader: WireReader): {{ enumeration.name() }} {
            val tag = reader.readU32()
            return when (tag) {
{%- for variant in enumeration.data_variants() %}
                {{ variant.tag() }} -> {{ variant.read() }}
{%- endfor %}
                else -> {{ enumeration.unknown_tag() }}
            }
        }

        internal fun fromByteArray(bytes: ByteArray): {{ enumeration.name() }} {
            val reader = WireReader(bytes)
            return fromReader(reader)
        }
    }
}
{%- endif %}
