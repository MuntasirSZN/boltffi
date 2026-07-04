{%- if enumeration.c_style() -%}
{{ enumeration.documentation() }}public enum {{ enumeration.name() }}: {{ enumeration.raw_type() }}, Hashable, Sendable, CaseIterable{{ enumeration.error_conformance() }} {
{%- for variant in enumeration.c_style_variants() %}
{{ variant.documentation() }}    case {{ variant.name() }} = {{ variant.discriminant() }}
{%- endfor %}

    @usableFromInline init(fromC c: {{ enumeration.raw_type() }}) {
        self = {{ enumeration.name() }}(rawValue: c)!
    }

    @usableFromInline var cValue: {{ enumeration.raw_type() }} {
        rawValue
    }
{%- endif %}
{%- if enumeration.data() -%}
{{ enumeration.documentation() }}public enum {{ enumeration.name() }}: Hashable, Equatable, Sendable{{ enumeration.error_conformance() }} {
{%- for variant in enumeration.data_variants() %}
{{ variant.documentation() }}    case {{ variant.name() }}{{ variant.payload().associated_values() }}
{%- endfor %}

    @inlinable static func decode(from reader: inout WireReader) -> {{ enumeration.name() }} {
        let tag = reader.readU32()
        switch tag {
{%- for variant in enumeration.data_variants() %}
        case {{ variant.tag() }}:
            return .{{ variant.name() }}{{ variant.payload().read_arguments() }}
{%- endfor %}
        default:
            fatalError("Invalid {{ enumeration.name() }} tag: \(tag)")
        }
    }

    @inlinable func encode(to writer: inout WireWriter) {
        switch self {
{%- for variant in enumeration.data_variants() %}
{%- if variant.payload().unit() %}
        case .{{ variant.name() }}:
            writer.writeU32({{ variant.tag() }})
{%- else %}
        case let .{{ variant.name() }}{{ variant.payload().case_pattern() }}:
            writer.writeU32({{ variant.tag() }})
{%- for field in variant.payload().fields() %}
            {{ field.write() }}
{%- endfor %}
{%- endif %}
{%- endfor %}
        }
    }
{%- endif %}
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
