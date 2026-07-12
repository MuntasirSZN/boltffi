

export const {{ name }} = {
{% for variant in variants %}  {{ variant.name }}: {{ variant.value }},
{% endfor %}} as const;

export type {{ name }} = typeof {{ name }}[keyof typeof {{ name }}];

const {{ codec }}: WireCodec<{{ name }}> = {
  size: () => {{ size }},
  encode: (writer, value) => {
    writer.{{ write }}(value);
  },
  decode: (reader) => {
    const value = reader.{{ read }}();
    switch (value) {
{% for variant in variants %}      case {{ variant.value }}: return {{ name }}.{{ variant.name }};
{% endfor %}      default: throw new Error(`Unknown {{ name }} wire tag: ${value}`);
    }
  },
};
