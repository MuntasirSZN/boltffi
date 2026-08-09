{%- match stream.owner() %}
{%- when Some with (owner) %}
extension on {{ owner }} {
{{ stream.associated_method() }}
}
{%- when None %}
{{ stream.method() }}
{%- endmatch %}
