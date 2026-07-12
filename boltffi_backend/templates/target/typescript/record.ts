

export interface {{ name }} {
{% for field in fields %}  readonly {{ field.key }}: {{ field.ty }};
{% endfor %}}

const {{ codec }}: WireCodec<{{ name }}> = {
  size: (value) => {{ size }},
  encode: (writer, value) => {
{% for statement in writes %}    {{ statement }}
{% endfor %}  },
  decode: (reader) => {
{% for statement in reads %}    {{ statement }}
{% endfor %}    return {
{% for field in fields %}      {{ field.key }}: {{ field.local }},
{% endfor %}    };
  },
};
