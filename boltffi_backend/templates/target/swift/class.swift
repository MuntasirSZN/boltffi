{{ class.documentation() }}public final class {{ class.name() }} {
    @usableFromInline let handle: {{ class.handle_type() }}

    @usableFromInline init(handle: {{ class.handle_type() }}) {
        self.handle = handle
    }

    deinit {
        {{ class.release() }}(handle)
    }
{%- for initializer in class.initializers() %}

{{ initializer.documentation() }}    public init({% for parameter in initializer.parameters() %}{{ parameter.signature() }}{% if !loop.last %}, {% endif %}{% endfor %}){{ initializer.throwing_keyword() }} {
{{ initializer.body() }}
    }
{%- endfor %}
{%- for method in class.static_methods() %}

{{ method.documentation() }}    public static func {{ method.name() }}({% for parameter in method.parameters() %}{{ parameter.signature() }}{% if !loop.last %}, {% endif %}{% endfor %}){{ method.returns().signature() }} {
{{ method.body() }}
    }
{%- endfor %}
{%- for method in class.instance_methods() %}

{{ method.documentation() }}    public func {{ method.name() }}({% for parameter in method.parameters() %}{{ parameter.signature() }}{% if !loop.last %}, {% endif %}{% endfor %}){{ method.returns().signature() }} {
{{ method.body() }}
    }
{%- endfor %}
}
