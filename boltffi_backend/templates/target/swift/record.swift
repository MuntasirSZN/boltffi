{{ record.documentation() }}public struct {{ record.name() }}: Hashable, Equatable, Sendable {
{%- for field in record.fields() %}
{{ field.documentation() }}    public var {{ field.name() }}: {{ field.ty() }}
{%- endfor %}

    public init({% for field in record.fields() %}{{ field.name() }}: {{ field.ty() }}{% if !loop.last %}, {% endif %}{% endfor %}) {
{%- for field in record.fields() %}
        {{ field.assignment() }}
{%- endfor %}
    }

    @usableFromInline init(fromC c: {{ record.c_type() }}) {
        self.init({% for field in record.fields() %}{{ field.c_initializer_argument() }}{% if !loop.last %}, {% endif %}{% endfor %})
    }

    @usableFromInline var cValue: {{ record.c_type() }} {
        {{ record.c_type() }}({% for field in record.fields() %}{{ field.c_value_argument() }}{% if !loop.last %}, {% endif %}{% endfor %})
    }
{%- for initializer in record.initializers() %}

{{ initializer.documentation() }}    public static func {{ initializer.name() }}({% for parameter in initializer.parameters() %}{{ parameter.signature() }}{% if !loop.last %}, {% endif %}{% endfor %}){{ initializer.returns().signature() }} {
{{ initializer.body() }}
    }
{%- endfor %}
{%- for method in record.static_methods() %}

{{ method.documentation() }}    public {{ method.static_keyword() }}func {{ method.name() }}({% for parameter in method.parameters() %}{{ parameter.signature() }}{% if !loop.last %}, {% endif %}{% endfor %}){{ method.returns().signature() }} {
{{ method.body() }}
    }
{%- endfor %}
{%- for method in record.instance_methods() %}

{{ method.documentation() }}    public {{ method.static_keyword() }}func {{ method.name() }}({% for parameter in method.parameters() %}{{ parameter.signature() }}{% if !loop.last %}, {% endif %}{% endfor %}){{ method.returns().signature() }} {
{{ method.body() }}
    }
{%- endfor %}
}
