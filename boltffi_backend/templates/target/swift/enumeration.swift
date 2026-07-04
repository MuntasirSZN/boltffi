{{ enumeration.documentation() }}public enum {{ enumeration.name() }}: {{ enumeration.raw_type() }}, Hashable, Sendable, CaseIterable {
{%- for variant in enumeration.variants() %}
{{ variant.documentation() }}    case {{ variant.name() }} = {{ variant.discriminant() }}
{%- endfor %}

    @usableFromInline init(fromC c: {{ enumeration.raw_type() }}) {
        self = {{ enumeration.name() }}(rawValue: c)!
    }

    @usableFromInline var cValue: {{ enumeration.raw_type() }} {
        rawValue
    }
{%- for initializer in enumeration.initializers() %}

{{ initializer.documentation() }}    public static func {{ initializer.name() }}({% for parameter in initializer.parameters() %}{{ parameter.signature() }}{% if !loop.last %}, {% endif %}{% endfor %}){{ initializer.returns().signature() }} {
{{ initializer.body() }}
    }
{%- endfor %}
{%- for method in enumeration.static_methods() %}

{{ method.documentation() }}    public {{ method.static_keyword() }}func {{ method.name() }}({% for parameter in method.parameters() %}{{ parameter.signature() }}{% if !loop.last %}, {% endif %}{% endfor %}){{ method.returns().signature() }} {
{{ method.body() }}
    }
{%- endfor %}
{%- for method in enumeration.instance_methods() %}

{{ method.documentation() }}    public {{ method.static_keyword() }}func {{ method.name() }}({% for parameter in method.parameters() %}{{ parameter.signature() }}{% if !loop.last %}, {% endif %}{% endfor %}){{ method.returns().signature() }} {
{{ method.body() }}
    }
{%- endfor %}
}
