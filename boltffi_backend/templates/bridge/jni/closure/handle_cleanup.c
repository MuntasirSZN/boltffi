{%- for bytes in handle.closure.handle_byte_arrays %}
    {{ free_buffer }}({{ bytes.buffer }});
{%- endfor %}
