{%- for bytes in closure.byte_arrays %}
    if ({{ bytes.name }} != NULL) {
        (*env)->DeleteLocalRef(env, {{ bytes.name }});
    }
{%- endfor %}
{%- for vector in closure.direct_vectors %}
    if ({{ vector.name }} != NULL) {
        (*env)->DeleteLocalRef(env, {{ vector.name }});
    }
{%- endfor %}
