const {{ finalizer }}: FinalizationRegistry<number> | null =
  typeof FinalizationRegistry === "undefined"
    ? null
    : new FinalizationRegistry<number>((handle) => {
        (_exports.{{ release }} as Function)(handle);
      });

export class {{ name }} {
  private _handle: number;
  private _disposed = false;

  private constructor(handle: number) {
    this._handle = handle;
    {{ finalizer }}?.register(this, handle, this);
  }

  static _fromHandle(handle: number): {{ name }} {
    if (handle === 0) {
      throw new Error("{{ name }} received a null handle");
    }
    return new {{ name }}(handle);
  }

  static _toHandle(value: {{ name }} | null): number {
    return value === null ? 0 : value._borrowHandle();
  }

  [Symbol.dispose](): void {
    this.dispose();
  }

  dispose(): void {
    if (this._disposed) {
      return;
    }
    this._disposed = true;
    {{ finalizer }}?.unregister(this);
    (_exports.{{ release }} as Function)(this._handle);
    this._handle = 0;
  }

  private _borrowHandle(): number {
    if (this._disposed) {
      throw new Error("{{ name }} has been disposed");
    }
    return this._handle;
  }
{% for method in methods %}
  {{ method }}
{% endfor %}}
