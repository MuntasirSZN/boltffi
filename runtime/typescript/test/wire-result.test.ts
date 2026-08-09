import { describe, expect, it } from "vitest";
import { matchWireResult, wireErr, wireOk } from "../src/wire.js";

const take = (value: unknown) =>
  matchWireResult(
    value as never,
    (ok) => ({ ok }),
    (err) => ({ err })
  );

describe("matchWireResult", () => {
  it("treats a binary payload as success", () => {
    // A callback returning `Result<Vec<u8>, E>` hands back a `Uint8Array`. It
    // carries no `tag`, so it used to reach the ambiguity throw; that throw was
    // reported as completion code -2 and decoded on the Rust side as the error
    // enum, which aborted the process instead of surfacing anything.
    const bytes = new Uint8Array([1, 2, 3]);

    expect(take(bytes)).toEqual({ ok: bytes });
    expect(take(new Float64Array([1.5]))).toEqual({ ok: new Float64Array([1.5]) });
    expect(take(new ArrayBuffer(4))).toEqual({ ok: new ArrayBuffer(4) });
  });

  it("still refuses a plain object", () => {
    // A record genuinely is ambiguous: it could be either side of the result,
    // so the caller has to say which.
    expect(() => take({ id: 1 })).toThrow(/Ambiguous/);
  });

  it("keeps the explicit wrappers working", () => {
    const bytes = new Uint8Array([9]);

    expect(take(wireOk(bytes))).toEqual({ ok: bytes });
    expect(take(wireErr("boom"))).toEqual({ err: "boom" });
  });

  it("keeps scalars and errors on their existing sides", () => {
    expect(take("text")).toEqual({ ok: "text" });
    expect(take(42)).toEqual({ ok: 42 });
    const failure = new Error("nope");
    expect(take(failure)).toEqual({ err: failure });
  });
});
