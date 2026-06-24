fun {{ function.name() }}({% for parameter in function.parameters() %}{{ parameter.name() }}: {{ parameter.ty() }}{% if !loop.last %}, {% endif %}{% endfor %}){% if let Some(return_type) = function.returns() %}: {{ return_type }}{% endif %} {
{%- for statement in function.setup() %}
    {{ statement }}
{%- endfor %}
{%- if function.has_cleanup() %}
    try {
{%- for statement in function.call() %}
        {{ statement }}
{%- endfor %}
    } finally {
{%- for statement in function.cleanup() %}
        {{ statement }}
{%- endfor %}
    }
{%- else %}
{%- for statement in function.call() %}
    {{ statement }}
{%- endfor %}
{%- endif %}
}
