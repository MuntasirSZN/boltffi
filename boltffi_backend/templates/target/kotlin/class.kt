class {{ class.name() }} internal constructor(internal val handle: Long) : AutoCloseable {
    private val __boltffi_closed = java.util.concurrent.atomic.AtomicBoolean(false)

    override fun close() {
        if (__boltffi_closed.compareAndSet(false, true)) {
            Native.{{ class.release() }}(handle)
        }
    }
{%- if !class.initializers().is_empty() || !class.static_methods().is_empty() %}

    companion object {
{%- for initializer in class.initializers() %}
        fun {{ initializer.call().name() }}({% for parameter in initializer.call().parameters() %}{{ parameter.name() }}: {{ parameter.ty() }}{% if !loop.last %}, {% endif %}{% endfor %}){% if let Some(return_type) = initializer.call().returns() %}: {{ return_type }}{% endif %} {
{%- for statement in initializer.call().setup() %}
            {{ statement }}
{%- endfor %}
{%- if initializer.call().has_cleanup() %}
            try {
{%- for statement in initializer.call().call() %}
                {{ statement }}
{%- endfor %}
            } finally {
{%- for statement in initializer.call().cleanup() %}
                {{ statement }}
{%- endfor %}
            }
{%- else %}
{%- for statement in initializer.call().call() %}
            {{ statement }}
{%- endfor %}
{%- endif %}
        }
{%- endfor %}
{%- for method in class.static_methods() %}
        fun {{ method.name() }}({% for parameter in method.parameters() %}{{ parameter.name() }}: {{ parameter.ty() }}{% if !loop.last %}, {% endif %}{% endfor %}){% if let Some(return_type) = method.returns() %}: {{ return_type }}{% endif %} {
{%- for statement in method.setup() %}
            {{ statement }}
{%- endfor %}
{%- if method.has_cleanup() %}
            try {
{%- for statement in method.call() %}
                {{ statement }}
{%- endfor %}
            } finally {
{%- for statement in method.cleanup() %}
                {{ statement }}
{%- endfor %}
            }
{%- else %}
{%- for statement in method.call() %}
            {{ statement }}
{%- endfor %}
{%- endif %}
        }
{%- endfor %}
    }
{%- endif %}
{%- for method in class.instance_methods() %}

    fun {{ method.name() }}({% for parameter in method.parameters() %}{{ parameter.name() }}: {{ parameter.ty() }}{% if !loop.last %}, {% endif %}{% endfor %}){% if let Some(return_type) = method.returns() %}: {{ return_type }}{% endif %} {
{%- for statement in method.setup() %}
        {{ statement }}
{%- endfor %}
{%- if method.has_cleanup() %}
        try {
{%- for statement in method.call() %}
            {{ statement }}
{%- endfor %}
        } finally {
{%- for statement in method.cleanup() %}
            {{ statement }}
{%- endfor %}
        }
{%- else %}
{%- for statement in method.call() %}
        {{ statement }}
{%- endfor %}
{%- endif %}
    }
{%- endfor %}
}
