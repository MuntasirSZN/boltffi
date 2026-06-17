static PyObject *{{ wrapper }}(PyObject *self, PyObject *const *args, Py_ssize_t nargs) {
{%- for param in params %}
{%- if param.is_direct() %}
    {{ param.c_type() }} {{ param.name() }};
{%- endif %}
{%- if param.is_encoded() %}
    PyObject *{{ param.wire() }} = NULL;
    const uint8_t *{{ param.pointer() }} = NULL;
    uintptr_t {{ param.length() }} = 0;
{%- endif %}
{%- endfor %}
{%- if let Some(fallible) = fallible %}
{%- if let Some(success_declaration) = fallible.success_declaration %}
    {{ success_declaration }};
{%- endif %}
    {{ fallible.error_type }} {{ fallible.error_value }} = {0};
    PyObject *error = NULL;
{%- endif %}
    PyObject *result = NULL;
    (void)self;
    if (nargs != {{ params.len() }}) {
        PyErr_Format(PyExc_TypeError, "{{ python_name }}() takes {{ params.len() }} positional arguments but %zd were given", nargs);
        goto done;
    }
    if ({{ storage }} == NULL) {
        PyErr_SetString(PyExc_ImportError, "native library is not initialized");
        goto done;
    }
{%- for param in params %}
{%- if param.is_direct() %}
    if (!{{ param.parser() }}(args[{{ param.index() }}], &{{ param.name() }})) {
        goto done;
    }
{%- endif %}
{%- if param.is_encoded() %}
    if (!{{ param.parser() }}(args[{{ param.index() }}], &{{ param.wire() }}, &{{ param.pointer() }}, &{{ param.length() }})) {
        goto done;
    }
{%- endif %}
{%- endfor %}
{%- if let Some(fallible) = fallible %}
    {{ fallible.error_value }} = {{ storage }}({%- for arg in call_args %}{{ arg }}{% if !loop.last %}, {% endif %}{%- endfor %});
    if ({{ fallible.error_value }}.len != 0) {
        error = {{ fallible.error.converter }}({{ fallible.error_value }});
        if (error != NULL) {
            PyErr_SetObject(PyExc_RuntimeError, error);
        }
        goto done;
    }
{%- if returns.is_void() %}
    Py_INCREF(Py_None);
    result = Py_None;
{%- else %}
    result = {{ returns.converter }}({{ fallible.success_value }});
{%- endif %}
{%- else %}
{%- if returns.is_void() %}
    {{ storage }}({%- for arg in call_args %}{{ arg }}{% if !loop.last %}, {% endif %}{%- endfor %});
    Py_INCREF(Py_None);
    result = Py_None;
{%- else %}
    result = {{ returns.converter }}({{ storage }}({%- for arg in call_args %}{{ arg }}{% if !loop.last %}, {% endif %}{%- endfor %}));
{%- endif %}
{%- endif %}
done:
{%- for param in params %}
{%- if param.is_encoded() %}
    Py_XDECREF({{ param.wire() }});
{%- endif %}
{%- endfor %}
{%- if fallible.is_some() %}
    Py_XDECREF(error);
{%- endif %}
    return result;
}
