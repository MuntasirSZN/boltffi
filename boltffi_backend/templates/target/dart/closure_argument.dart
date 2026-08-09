{% if registration.nullable() %}{{ registration.callable_type }} {{ registration.call_callable }} = null;
{{ registration.release_callable_type }} {{ registration.release_callable }} = null;
{% else %}late final {{ registration.callable_type }} {{ registration.call_callable }};
late final {{ registration.release_callable_type }} {{ registration.release_callable }};
{% endif %}{{ registration.invoke_return }} {{ registration.invoke_function }}(
{%- for parameter in registration.invoke_parameters %}
  {{ parameter }}{% if !loop.last %},{% endif %}
{%- endfor %}
) {
{{ registration.invoke_body }}
}
void {{ registration.release_function }}($$ffi.Pointer<$$ffi.Void> _) {
  {{ registration.call_callable }}{% if registration.nullable() %}?{% endif %}.close();
  {{ registration.release_callable }}{% if registration.nullable() %}?{% endif %}.close();
}
{% if registration.nullable() %}if ({{ registration.source }} != null) {
  {{ registration.call_callable }} = $$ffi.NativeCallable<{{ registration.native_signature }}>.isolateLocal({{ registration.invoke_function }}{% if let Some(exceptional_return) = registration.exceptional_return %}, exceptionalReturn: {{ exceptional_return }}{% endif %});
  {{ registration.release_callable }} = $$ffi.NativeCallable<{{ registration.release_signature }}>.listener({{ registration.release_function }});
}{% else %}{{ registration.call_callable }} = $$ffi.NativeCallable<{{ registration.native_signature }}>.isolateLocal({{ registration.invoke_function }}{% if let Some(exceptional_return) = registration.exceptional_return %}, exceptionalReturn: {{ exceptional_return }}{% endif %});
{{ registration.release_callable }} = $$ffi.NativeCallable<{{ registration.release_signature }}>.listener({{ registration.release_function }});{% endif %}
