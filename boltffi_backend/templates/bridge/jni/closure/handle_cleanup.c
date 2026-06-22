{%- for bytes in handle.closure.handle_byte_arrays %}
    {{ free_buffer }}({{ bytes.buffer }});
{%- endfor %}
{%- for vector in handle.closure.handle_direct_vectors %}
    if ({{ vector.pointer_local }} != NULL) {
        (*env)->{{ vector.releaser }}(env, {{ vector.name }}, {{ vector.pointer_local }}, JNI_ABORT);
    }
{%- endfor %}
