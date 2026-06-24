{%- if record.empty() %}
object {{ record.name() }} {
{%- if record.encoded() %}
    internal fun wireSize(): Int = 0

    internal fun writeTo(writer: WireWriter) {
    }

    internal fun toByteArray(): ByteArray = ByteArray(0)

    internal fun fromReader(reader: WireReader): {{ record.name() }} {
        return {{ record.name() }}
    }

    internal fun fromByteArray(bytes: ByteArray): {{ record.name() }} {
        require(bytes.size == 0)
        return {{ record.name() }}
    }
{%- else %}
    internal fun toByteArray(): ByteArray = ByteArray(0)

    internal fun fromByteArray(bytes: ByteArray): {{ record.name() }} {
        require(bytes.size == 0)
        return {{ record.name() }}
    }
{%- endif %}
}
{%- else if record.encoded() %}
data class {{ record.name() }}(
{%- for field in record.fields() %}
    val {{ field.name() }}: {{ field.ty() }}{% if !loop.last %},{% endif %}
{%- endfor %}
) {
    internal fun wireSize(): Int {
{%- if let Some(wire_size) = record.wire_size() %}
        return {{ wire_size }}
{%- endif %}
    }

    internal fun writeTo(writer: WireWriter) {
{%- for field in record.fields() %}
        {{ field.write() }}
{%- endfor %}
    }

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

    companion object {
        internal fun fromReader(reader: WireReader): {{ record.name() }} {
            return {{ record.name() }}(
{%- for field in record.fields() %}
                {{ field.read() }}{% if !loop.last %},{% endif %}
{%- endfor %}
            )
        }

        internal fun fromByteArray(bytes: ByteArray): {{ record.name() }} {
            val reader = WireReader(bytes)
            return fromReader(reader)
        }
    }
}
{%- else %}
data class {{ record.name() }}(
{%- for field in record.fields() %}
    val {{ field.name() }}: {{ field.ty() }}{% if !loop.last %},{% endif %}
{%- endfor %}
) {
    internal fun toByteArray(): ByteArray {
        val buffer = java.nio.ByteBuffer
            .allocate({{ record.size() }})
            .order(java.nio.ByteOrder.nativeOrder())
{%- for field in record.fields() %}
        {{ field.write() }}
{%- endfor %}
        return buffer.array()
    }

    companion object {
        internal fun fromByteArray(bytes: ByteArray): {{ record.name() }} {
            require(bytes.size == {{ record.size() }})
            val buffer = java.nio.ByteBuffer
                .wrap(bytes)
                .order(java.nio.ByteOrder.nativeOrder())
            return {{ record.name() }}(
{%- for field in record.fields() %}
                {{ field.read() }}{% if !loop.last %},{% endif %}
{%- endfor %}
            )
        }
    }
}
{%- endif %}
