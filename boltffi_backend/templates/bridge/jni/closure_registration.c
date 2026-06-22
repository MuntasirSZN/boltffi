{%- for closure in closures %}

static jclass {{ closure.global_class }} = NULL;
static jmethodID {{ closure.call_method }} = NULL;
static jmethodID {{ closure.free_method }} = NULL;

static {{ closure.c_return_type }} {{ closure.call }}(void *user_data{% for argument in closure.arguments %}, {{ argument.c_type }} {{ argument.name }}{% endfor %}) {
    JNIEnv *env = NULL;
    int attached = 0;
    if (!boltffi_jni_enter(&env, &attached)) {
{%- if closure.returns_void %}
        return;
{%- else %}
        return {{ closure.failure_value }};
{%- endif %}
    }
    jlong handle = (jlong)(uintptr_t)user_data;
{%- if closure.returns_void %}
    (*env)->CallStaticVoidMethod(env, {{ closure.global_class }}, {{ closure.call_method }}, handle{% for argument in closure.arguments %}, ({{ argument.jni_type }}){{ argument.name }}{% endfor %});
    boltffi_jni_clear_exception(env);
    boltffi_jni_exit(attached);
{%- else %}
    {{ closure.c_return_type }} result = ({{ closure.c_return_type }})(*env)->CallStatic{{ closure.call_method_suffix }}Method(env, {{ closure.global_class }}, {{ closure.call_method }}, handle{% for argument in closure.arguments %}, ({{ argument.jni_type }}){{ argument.name }}{% endfor %});
    if (boltffi_jni_clear_exception(env)) {
        boltffi_jni_exit(attached);
        return {{ closure.failure_value }};
    }
    boltffi_jni_exit(attached);
    return result;
{%- endif %}
}

static void {{ closure.release }}(void *user_data) {
    JNIEnv *env = NULL;
    int attached = 0;
    if (!boltffi_jni_enter(&env, &attached)) {
        return;
    }
    (*env)->CallStaticVoidMethod(env, {{ closure.global_class }}, {{ closure.free_method }}, (jlong)(uintptr_t)user_data);
    boltffi_jni_clear_exception(env);
    boltffi_jni_exit(attached);
}

static bool {{ closure.load }}(JNIEnv *env) {
    jclass local_class = (*env)->FindClass(env, {{ closure.class }});
    if (local_class == NULL) {
        return false;
    }
    {{ closure.global_class }} = (*env)->NewGlobalRef(env, local_class);
    (*env)->DeleteLocalRef(env, local_class);
    if ({{ closure.global_class }} == NULL) {
        return false;
    }
    {{ closure.call_method }} = (*env)->GetStaticMethodID(env, {{ closure.global_class }}, "call", {{ closure.method_signature }});
    if ({{ closure.call_method }} == NULL) {
        goto fail;
    }
    {{ closure.free_method }} = (*env)->GetStaticMethodID(env, {{ closure.global_class }}, "free", "(J)V");
    if ({{ closure.free_method }} == NULL) {
        goto fail;
    }
    return true;
fail:
    (*env)->DeleteGlobalRef(env, {{ closure.global_class }});
    {{ closure.global_class }} = NULL;
    return false;
}

static void {{ closure.unload }}(JNIEnv *env) {
    if ({{ closure.global_class }} != NULL) {
        (*env)->DeleteGlobalRef(env, {{ closure.global_class }});
    }
    {{ closure.global_class }} = NULL;
    {{ closure.call_method }} = NULL;
    {{ closure.free_method }} = NULL;
}
{%- endfor %}
