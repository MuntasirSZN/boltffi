import { describe, expect, it } from "vitest";
import { WireReader, WireWriter, writeUnexpectedCallbackError } from "../src/wire.js";

const MARKER = "BOLTFFI_CALLBACK";

/** Stands in for the wasm module: hands out a plain writer and records the size asked for. */
function allocator(): { allocWriter(size: number): WireWriter; requested: number[] } {
  const requested: number[] = [];
  return {
    requested,
    allocWriter(size: number) {
      requested.push(size);
      return new WireWriter();
    },
  };
}

function bytesOf(writer: WireWriter): Uint8Array {
  return writer.getBytes();
}

/**
 * Mirrors `UnexpectedFfiCallbackError::classify_payload` in `boltffi_core`.
 * The envelope is only useful if that function reads back what this writes, so
 * the assertions here are that decoder's steps rather than a byte snapshot.
 */
function classify(payload: Uint8Array): string {
  const marker = new TextDecoder().decode(payload.subarray(0, MARKER.length));
  expect(marker).toBe(MARKER);
  expect(payload[MARKER.length]).toBe(1);

  const reader = new WireReader(
    payload.buffer as ArrayBuffer,
    payload.byteOffset + MARKER.length + 1
  );
  const message = reader.readString();
  // `classify_payload` rejects trailing bytes, so the message must end the payload.
  expect(MARKER.length + 1 + 4 + new TextEncoder().encode(message).length).toBe(payload.length);
  return message;
}

describe("writeUnexpectedCallbackError", () => {
  it("wraps an Error message in the envelope Rust classifies", () => {
    const writer = writeUnexpectedCallbackError(allocator(), new Error("boom from js"));
    expect(classify(bytesOf(writer))).toBe("boom from js");
  });

  it("stringifies a thrown non-Error", () => {
    const writer = writeUnexpectedCallbackError(allocator(), "just a string");
    expect(classify(bytesOf(writer))).toBe("just a string");
  });

  it("keeps a multi-byte message intact", () => {
    // The size request is in bytes but the message is measured in JS chars,
    // so an under-sized request would surface as a truncated or throwing write.
    const message = "não pôde: 日本語 🎉";
    const writer = writeUnexpectedCallbackError(allocator(), new Error(message));
    expect(classify(bytesOf(writer))).toBe(message);
  });

  it("asks for exactly the space the envelope occupies", () => {
    const alloc = allocator();
    const writer = writeUnexpectedCallbackError(alloc, new Error("é"));
    expect(alloc.requested).toEqual([bytesOf(writer).length]);
  });

  it("carries an empty message rather than an empty payload", () => {
    // An empty payload is how cancellation reports itself; a thrown error with
    // no message must not be mistaken for one.
    const writer = writeUnexpectedCallbackError(allocator(), new Error(""));
    const bytes = bytesOf(writer);
    expect(bytes.length).toBeGreaterThan(0);
    expect(classify(bytes)).toBe("");
  });
});
