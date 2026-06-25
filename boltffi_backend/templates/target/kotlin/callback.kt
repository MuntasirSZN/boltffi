interface {{ callback.name() }} {
{%- for method in callback.methods() %}
    fun {{ method.name() }}({% for parameter in method.public_parameters() %}{{ parameter.name() }}: {{ parameter.ty() }}{% if !loop.last %}, {% endif %}{% endfor %}){% if let Some(return_type) = method.public_return() %}: {{ return_type }}{% endif %}
{%- endfor %}
}

private object {{ callback.map_name() }} {
    private val map = java.util.concurrent.ConcurrentHashMap<Long, {{ callback.name() }}>()
    private val counter = java.util.concurrent.atomic.AtomicLong(1L)

    fun insert(value: {{ callback.name() }}): Long {
        val handle = counter.getAndAdd(2L)
        map[handle] = value
        return handle
    }

    fun get(handle: Long): {{ callback.name() }}? = map[handle]

    fun remove(handle: Long): {{ callback.name() }}? = map.remove(handle)

    fun clone(handle: Long): Long {
        val value = map[handle] ?: return 0L
        return insert(value)
    }
}

object {{ callback.callbacks_name() }} {
    @JvmStatic
    fun free(handle: Long) {
        {{ callback.map_name() }}.remove(handle)
    }

    @JvmStatic
    fun clone(handle: Long): Long {
        return {{ callback.map_name() }}.clone(handle)
    }
{%- for method in callback.methods() %}

    @JvmStatic
    fun {{ method.jvm_name() }}(handle: Long{% for parameter in method.jvm_parameters() %}, {{ parameter.name() }}: {{ parameter.ty() }}{% endfor %}){% if let Some(return_type) = method.jvm_return() %}: {{ return_type }}{% endif %} {
        val impl = {{ callback.map_name() }}.get(handle) ?: error("{{ callback.map_name() }}: invalid handle $handle")
{%- for statement in method.setup() %}
        {{ statement }}
{%- endfor %}
{%- for statement in method.call_return() %}
        {{ statement }}
{%- endfor %}
    }
{%- endfor %}
}

object {{ callback.bridge_name() }} {
    fun create(value: {{ callback.name() }}): Long {
        return {{ callback.map_name() }}.insert(value)
    }
}
