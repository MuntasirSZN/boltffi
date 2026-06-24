fun {{ function.name() }}({% for parameter in function.parameters() %}{{ parameter.name() }}: {{ parameter.ty() }}{% if !loop.last %}, {% endif %}{% endfor %}){% if let Some(return_type) = function.returns() %}: {{ return_type }}{% endif %} {
{%- for statement in function.body() %}
    {{ statement }}
{%- endfor %}
}
