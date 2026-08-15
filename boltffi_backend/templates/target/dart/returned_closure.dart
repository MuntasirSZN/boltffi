final {{ registration.registration }} = {{ registration.storage }}.ptr.ref;
{% if registration.nullable() %}final {{ registration.returned }} = {{ registration.registration }}.invoke == $$ffi.nullptr ? null : (() {
  final {{ registration.owner }} = _$$BoltReturnedClosureOwner({{ registration.registration }});
  return (
{%- for parameter in registration.parameters %}
    {{ parameter }}{% if !loop.last %},{% endif %}
{%- endfor %}
  ) {
{{ registration.nested_body() }}
  };
})();
{% else %}if ({{ registration.registration }}.invoke == $$ffi.nullptr) { throw StateError('Rust returned a null required closure'); }
final {{ registration.owner }} = _$$BoltReturnedClosureOwner({{ registration.registration }});
final {{ registration.returned }} = (
{%- for parameter in registration.parameters %}
  {{ parameter }}{% if !loop.last %},{% endif %}
{%- endfor %}
) {
{{ registration.body() }}
};
{% endif %}
