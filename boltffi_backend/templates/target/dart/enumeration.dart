{%- if enumeration.c_style() -%}
{{ enumeration.documentation() }}enum {{ enumeration.name() }}{{ enumeration.exception_clause() }} {
{%- for variant in enumeration.c_style_body().variants() %}
{{ variant.documentation() }}  {{ variant.name() }}({{ variant.discriminant() }}){% if loop.last %};{% else %},{% endif %}
{%- endfor %}

  final int value;
  const {{ enumeration.name() }}(this.value);

  static {{ enumeration.name() }} _m$fromDiscriminant(int value) => values.firstWhere(
    (variant) => variant.value == value,
    orElse: () => throw ArgumentError.value(
      value,
      'value',
      'unknown {{ enumeration.name() }} discriminant',
    ),
  );

  static {{ enumeration.name() }} _m$wireDecode(_$$BoltWireDecoder _p$reader) =>
      _m$fromDiscriminant(_p$reader.{{ enumeration.c_style_body().read_method() }}());

  void _m$wireEncode(_$$BoltWireEncoder _p$writer) =>
      _p$writer.{{ enumeration.c_style_body().write_method() }}(value);

  int _m$wireEncodedSize() => {{ enumeration.c_style_body().encoded_size() }};
{%- for member in enumeration.members() %}

{{ member }}
{%- endfor %}
}
{%- endif %}
{%- if enumeration.data() -%}
{{ enumeration.documentation() }}sealed class {{ enumeration.name() }}{{ enumeration.exception_clause() }} {
  const {{ enumeration.name() }}();
{%- for variant in enumeration.data_body().variants() %}

{{ variant.member_documentation() }}{% if variant.unit() %}  static const {{ variant.name() }} = {{ variant.class_name() }}();
{%- else %}
  static {{ enumeration.name() }} {{ variant.name() }}({
{%- for field in variant.fields() %}
    required {{ field.ty() }} {{ field.name() }},{% endfor %}
  }) => {{ variant.class_name() }}(
{%- for field in variant.fields() %}
    {{ field.name() }}: {{ field.name() }},{% endfor %}
  );
{%- endif %}
{%- endfor %}

  static {{ enumeration.name() }} _m$wireDecode(_$$BoltWireDecoder _p$reader) {
    return switch (_p$reader.readU32()) {
{%- for variant in enumeration.data_body().variants() %}
      {{ variant.tag() }} => {{ variant.class_name() }}(
{%- for field in variant.fields() %}
        {{ field.name() }}: {{ field.read() }},{% endfor %}
      ),
{%- endfor %}
      final tag => throw ArgumentError.value(
        tag,
        'tag',
        'unknown {{ enumeration.name() }} tag',
      ),
    };
  }

  void _m$wireEncode(_$$BoltWireEncoder _p$writer);
  int _m$wireEncodedSize();
{%- for member in enumeration.members() %}

{{ member }}
{%- endfor %}
}
{%- for variant in enumeration.data_body().variants() %}

{{ variant.declaration_documentation() }}final class {{ variant.class_name() }} extends {{ enumeration.name() }} {
{%- for field in variant.fields() %}
  final {{ field.ty() }} {{ field.name() }};
{%- endfor %}

  const {{ variant.class_name() }}({
{%- for field in variant.fields() %}
    required this.{{ field.name() }},{% endfor %}
  });

  @override
  void _m$wireEncode(_$$BoltWireEncoder _p$writer) {
    _p$writer.writeU32({{ variant.tag() }});
{%- for field in variant.fields() %}
{%- for write in field.writes() %}
    {{ write }}
{%- endfor %}
{%- endfor %}
  }

  @override
  int _m$wireEncodedSize() => {{ variant.encoded_size() }};
}
{%- endfor %}
{%- endif %}
