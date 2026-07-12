package {{ package }};

import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;

{% if let Some(doc) = callback.doc() %}{{ doc }}
{% endif %}public interface {{ callback.name() }} {
{% for method in callback.methods() %}{% if let Some(doc) = method.doc() %}{{ doc }}
{% endif %}    {{ method.public_return() }} {{ method.name() }}({% for parameter in method.public_parameters() %}{{ parameter.ty() }} {{ parameter.name() }}{% if !loop.last %}, {% endif %}{% endfor %});
{% endfor %}}

final class {{ callback.callbacks_name() }} {
    private static final ConcurrentHashMap<Long, {{ callback.name() }}> VALUES = new ConcurrentHashMap<>();
    private static final AtomicLong NEXT = new AtomicLong(1L);

    private {{ callback.callbacks_name() }}() {}

    static long insert({{ callback.name() }} value) {
        long handle = NEXT.getAndAdd(2L);
        VALUES.put(handle, value);
        return handle;
    }

    static {{ callback.name() }} get(long handle) {
        return VALUES.get(handle);
    }

    static void free(long handle) {
        VALUES.remove(handle);
    }

    static long clone(long handle) {
        {{ callback.name() }} value = VALUES.get(handle);
        return value == null ? 0L : insert(value);
    }
{% for method in callback.methods() %}
    static {{ method.jvm_return() }} {{ method.jvm_name() }}(long handle{% for parameter in method.jvm_parameters() %}, {{ parameter.ty() }} {{ parameter.name() }}{% endfor %}) {
        {{ callback.name() }} implementation = VALUES.get(handle);
        if (implementation == null) throw new IllegalStateException("invalid callback handle");
{% for statement in method.setup() %}        {{ statement }}
{% endfor %}{% for statement in method.body() %}        {{ statement }}
{% endfor %}    }
{% endfor %}}
{% if !callback.handle_methods().is_empty() %}
final class {{ callback.handle_name() }} implements {{ callback.name() }}, AutoCloseable {
    private final long handle;
    private final AtomicBoolean closed = new AtomicBoolean(false);

    {{ callback.handle_name() }}(long handle) {
        this.handle = handle;
    }

    long rawHandle() {
        return handle;
    }

    @Override
    public void close() {
        if (!closed.compareAndSet(false, true)) return;
{% if let Some(release) = callback.handle_release() %}        Native.{{ release }}(handle);
{% endif %}    }
{% for method in callback.handle_methods() %}
    @Override
    public {{ method.returns() }} {{ method.name() }}({% for parameter in method.parameters() %}{{ parameter.ty() }} {{ parameter.name() }}{% if !loop.last %}, {% endif %}{% endfor %}) {
{% for statement in method.body() %}        {{ statement }}
{% endfor %}    }
{% endfor %}}
{% endif %}

final class {{ callback.bridge_name() }} {
    private {{ callback.bridge_name() }}() {}

    static long create({{ callback.name() }} value) {
{% if !callback.handle_methods().is_empty() %}        if (value instanceof {{ callback.handle_name() }}) {
{% if let Some(clone) = callback.handle_clone() %}            return Native.{{ clone }}((({{ callback.handle_name() }}) value).rawHandle());
{% endif %}        }
{% endif %}
        return {{ callback.callbacks_name() }}.insert(value);
    }

    static {{ callback.name() }} wrap(long handle) {
        {{ callback.name() }} existing = {{ callback.callbacks_name() }}.get(handle);
        if (existing != null) return existing;
{% if !callback.handle_methods().is_empty() %}        return new {{ callback.handle_name() }}(handle);
{% else %}        throw new IllegalStateException("callback handle has no native methods");
{% endif %}
    }
}
