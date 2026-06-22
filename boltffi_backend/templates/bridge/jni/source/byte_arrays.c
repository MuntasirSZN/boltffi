static jbyteArray boltffi_jni_buffer_to_byte_array(JNIEnv *env, FfiBuf_u8 buffer) {
    if (buffer.ptr == NULL) {
        if (buffer.len != 0) {
            boltffi_jni_throw_runtime(env, "BoltFFI buffer pointer was null with non-zero length");
        }
        return NULL;
    }
    if (buffer.len > (uintptr_t)INT32_MAX) {
        {{ free_buffer }}(buffer);
        boltffi_jni_throw_runtime(env, "BoltFFI buffer too large for Java byte array");
        return NULL;
    }
    jbyteArray array = (*env)->NewByteArray(env, (jsize)buffer.len);
    if (array == NULL) {
        {{ free_buffer }}(buffer);
        return NULL;
    }
    (*env)->SetByteArrayRegion(env, array, 0, (jsize)buffer.len, (const jbyte *)buffer.ptr);
    {{ free_buffer }}(buffer);
    if ((*env)->ExceptionCheck(env)) {
        (*env)->DeleteLocalRef(env, array);
        return NULL;
    }
    return array;
}

static jbyteArray boltffi_jni_bytes_to_byte_array(JNIEnv *env, const uint8_t *bytes, uintptr_t len) {
    if (bytes == NULL && len != 0) {
        boltffi_jni_throw_runtime(env, "BoltFFI byte slice pointer was null with non-zero length");
        return NULL;
    }
    if (len > (uintptr_t)INT32_MAX) {
        boltffi_jni_throw_runtime(env, "BoltFFI byte slice too large for Java byte array");
        return NULL;
    }
    jbyteArray array = (*env)->NewByteArray(env, (jsize)len);
    if (array == NULL) {
        return NULL;
    }
    if (len != 0) {
        (*env)->SetByteArrayRegion(env, array, 0, (jsize)len, (const jbyte *)bytes);
        if ((*env)->ExceptionCheck(env)) {
            (*env)->DeleteLocalRef(env, array);
            return NULL;
        }
    }
    return array;
}

static FfiBuf_u8 boltffi_jni_byte_array_to_buffer(JNIEnv *env, jbyteArray array) {
    FfiBuf_u8 empty = {0};
    if (array == NULL) {
        boltffi_jni_throw_runtime(env, "BoltFFI byte array return was null");
        return empty;
    }
    jsize len = (*env)->GetArrayLength(env, array);
    if (len == 0) {
        return empty;
    }
    uint8_t *bytes = (uint8_t *)malloc((size_t)len);
    if (bytes == NULL) {
        boltffi_jni_throw_runtime(env, "failed to allocate BoltFFI byte array return");
        return empty;
    }
    (*env)->GetByteArrayRegion(env, array, 0, len, (jbyte *)bytes);
    if ((*env)->ExceptionCheck(env)) {
        free(bytes);
        return empty;
    }
    FfiBuf_u8 buffer = {
        .ptr = bytes,
        .len = (uintptr_t)len,
        .cap = (uintptr_t)len,
        .align = 1,
    };
    return buffer;
}
