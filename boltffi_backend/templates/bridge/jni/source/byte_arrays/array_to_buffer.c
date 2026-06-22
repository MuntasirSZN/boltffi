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
