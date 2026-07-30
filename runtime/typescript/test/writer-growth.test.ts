/**
 * Correctness gate for the `WireWriter` view cache (patch p1).
 *
 * The cache is validated by a detachment probe rather than by comparing
 * `allocator.buffer()` against the last buffer, so the one thing that can break
 * it is wasm memory growing while a writer is live. Growth detaches the old
 * ArrayBuffer, so the probe's `byteLength` drops to 0 and the caches refresh.
 *
 * Run from runtime/typescript with this file copied into test/, or with
 *   npx vitest run --root . <path>
 *
 * It has teeth: with the probe check replaced by an unconditional
 * `return this.cachedWasmView`, "writes land in the new buffer after growth"
 * fails with a detached-ArrayBuffer TypeError.
 */
import { describe, expect, it } from "vitest";
import { WireReader, WireWriter } from "../src/wire.js";
import type { WasmWireWriterAllocator } from "../src/wire.js";

function wasmBackedAllocator(memory: WebAssembly.Memory, bumpStart = 64) {
  let bump = bumpStart;
  const allocator: WasmWireWriterAllocator = {
    alloc: (size) => {
      const pointer = bump;
      bump = (bump + size + 7) & ~7;
      return pointer;
    },
    realloc: (_ptr, _old, newSize) => {
      const pointer = bump;
      bump = (bump + newSize + 7) & ~7;
      return pointer;
    },
    free: () => {},
    // Exactly the shape module.ts installs: an accessor read per call.
    buffer: () => memory.buffer,
  };
  return allocator;
}

describe("WireWriter over wasm memory that grows", () => {
  it("writes land in the new buffer after growth", () => {
    const memory = new WebAssembly.Memory({ initial: 1 });
    const allocator = wasmBackedAllocator(memory);
    const writer = WireWriter.withWasmAllocation(64, allocator);

    // Populate the cache against the pre-growth buffer.
    writer.writeU32(0xaaaa);
    const before = memory.buffer;

    memory.grow(2);
    expect(memory.buffer).not.toBe(before);
    expect(before.byteLength).toBe(0); // detached, which is what the probe sees

    // Same writer, same pointer, new buffer.
    writer.writeU32(0xbbbb);

    const bytes = new Uint8Array(memory.buffer);
    const reader = new WireReader(memory.buffer, writer.ptr);
    expect(reader.readU32()).toBe(0xaaaa);
    expect(reader.readU32()).toBe(0xbbbb);
    expect(bytes.byteLength).toBe(3 * 65536);
  });

  it("keeps every field of a record correct across a growth mid-encode", () => {
    const memory = new WebAssembly.Memory({ initial: 1 });
    const writer = WireWriter.withWasmAllocation(256, wasmBackedAllocator(memory));

    const values = [1, 2, 3, 4, 5, 6, 7, 8];
    values.slice(0, 4).forEach((v) => writer.writeI32(v));
    memory.grow(1);
    values.slice(4).forEach((v) => writer.writeI32(v));

    const reader = new WireReader(memory.buffer, writer.ptr);
    expect(values.map(() => reader.readI32())).toEqual(values);
  });

  it("refreshes the buffer used by writeBytes after growth", () => {
    const memory = new WebAssembly.Memory({ initial: 1 });
    const writer = WireWriter.withWasmAllocation(256, wasmBackedAllocator(memory));

    writer.writeU32(7); // seed the cache
    memory.grow(1);
    writer.writeBytes(Uint8Array.from([9, 8, 7, 6]));

    const reader = new WireReader(memory.buffer, writer.ptr);
    expect(reader.readU32()).toBe(7);
    expect([...reader.readBytes()]).toEqual([9, 8, 7, 6]);
  });

  it("a region-bound writer sharing one allocator also refreshes", () => {
    const memory = new WebAssembly.Memory({ initial: 1 });
    const allocator = wasmBackedAllocator(memory);
    const first = WireWriter.withWasmRegion(128, 64, allocator.buffer, allocator);
    first.writeU32(0x1111);
    memory.grow(1);
    // A second writer over the same shared allocator, after growth.
    const second = WireWriter.withWasmRegion(256, 64, allocator.buffer, allocator);
    second.writeU32(0x2222);
    first.writeU32(0x3333);

    expect(new WireReader(memory.buffer, 128).readU32()).toBe(0x1111);
    const firstReader = new WireReader(memory.buffer, 128);
    firstReader.readU32();
    expect(firstReader.readU32()).toBe(0x3333);
    expect(new WireReader(memory.buffer, 256).readU32()).toBe(0x2222);
  });
});
