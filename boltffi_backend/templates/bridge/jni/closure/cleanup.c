{%- for bytes in closure.byte_arrays %}
    if ({{ bytes.name }} != NULL) {
        (*env)->DeleteLocalRef(env, {{ bytes.name }});
    }
{%- endfor %}
