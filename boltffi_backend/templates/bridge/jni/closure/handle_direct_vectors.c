{%- for vector in handle.closure.handle_direct_vectors %}
    if ({{ vector.name }} == NULL) {
        boltffi_jni_throw_illegal_argument(env, "BoltFFI array argument was null");
{% include "bridge/jni/closure/handle_cleanup.c" %}
{% include "bridge/jni/closure/handle_fail.c" %}
    }
    {{ vector.length_local }} = (*env)->GetArrayLength(env, {{ vector.name }});
    {{ vector.pointer_local }} = (*env)->{{ vector.getter }}(env, {{ vector.name }}, NULL);
    if ({{ vector.pointer_local }} == NULL) {
{% include "bridge/jni/closure/handle_cleanup.c" %}
{% include "bridge/jni/closure/handle_fail.c" %}
    }
{%- endfor %}
