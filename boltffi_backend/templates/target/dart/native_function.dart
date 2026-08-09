@$$ffi.Native<{{ function.returns().native() }} Function(
{%- for parameter in function.parameters() -%}
{{ parameter.ty().native() }}{% if !loop.last %}, {% endif %}
{%- endfor -%}
)>(
  symbol: '{{ function.name() }}'{% if function.leaf() %},
  isLeaf: true{% endif %},
)
external {{ function.returns().dart() }} _f${{ function.name() }}(
{%- for parameter in function.parameters() %}
  {{ parameter.ty().dart() }} {{ parameter.name() }}{% if !loop.last %},{% endif %}
{%- endfor %}
);
