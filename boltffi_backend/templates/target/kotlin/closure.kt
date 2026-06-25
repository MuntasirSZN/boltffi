private object {{ closure.name() }} {
    private val map = java.util.concurrent.ConcurrentHashMap<Long, {{ closure.function_type() }}>()
    private val counter = java.util.concurrent.atomic.AtomicLong(1L)

    fun insert(value: {{ closure.function_type() }}): Long {
        val handle = counter.getAndAdd(2L)
        map[handle] = value
        return handle
    }

    @JvmStatic
    fun free(handle: Long) {
        map.remove(handle)
    }

    @JvmStatic
    fun call(handle: Long{% for parameter in closure.parameters() %}, {{ parameter.name() }}: {{ parameter.ty() }}{% endfor %}){% if let Some(return_type) = closure.returns() %}: {{ return_type }}{% endif %} {
        val impl = map[handle] ?: error("{{ closure.name() }}: invalid handle $handle")
{%- for statement in closure.setup() %}
        {{ statement }}
{%- endfor %}
{%- for statement in closure.call() %}
        {{ statement }}
{%- endfor %}
    }
}
