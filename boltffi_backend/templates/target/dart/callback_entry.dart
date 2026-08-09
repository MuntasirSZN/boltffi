static {{ return_type }} {{ name }}({% for parameter in parameters %}{{ parameter }}{% if !loop.last %}, {% endif %}{% endfor %}){% if asynchronous %} async{% endif %} {
{{ body }}
}
