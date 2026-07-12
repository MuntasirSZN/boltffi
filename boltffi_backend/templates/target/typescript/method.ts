{{ name }}({% for parameter in parameters %}{{ parameter.name }}: {{ parameter.ty }}{% if !loop.last %}, {% endif %}{% endfor %}): {{ returns }} {
{% for statement in body %}  {{ statement }}
{% endfor %}},
