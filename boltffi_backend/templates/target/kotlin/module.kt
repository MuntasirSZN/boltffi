@file:OptIn(kotlin.ExperimentalUnsignedTypes::class)

package {{ package }}

{{ runtime }}

@Suppress("FunctionName")
private object Native {
{%- for function in native_functions %}
    @JvmStatic external fun {{ function.name() }}({% for parameter in function.parameters() %}{{ parameter.name() }}: {{ parameter.ty() }}{% if !loop.last %}, {% endif %}{% endfor %}): {{ function.returns() }}
{%- endfor %}
{%- if async_runtime %}

    @JvmStatic fun boltffiFutureContinuationCallback(handle: Long, pollResult: Byte) {
        BoltFfiAsync.resume(handle, pollResult)
    }
{%- endif %}
}
{%- if !closures.is_empty() %}

{{ closures }}
{%- endif %}

{{ declarations }}
