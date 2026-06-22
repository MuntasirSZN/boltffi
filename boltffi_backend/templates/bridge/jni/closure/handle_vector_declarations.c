{%- for vector in handle.closure.handle_direct_vectors %}
    {{ vector.element_type }} *{{ vector.pointer_local }} = NULL;
    jsize {{ vector.length_local }} = 0;
{%- endfor %}
