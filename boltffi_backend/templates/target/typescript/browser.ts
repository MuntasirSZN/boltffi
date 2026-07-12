import { BoltFFIModule, instantiateBoltFFI, utf8ByteCount, wireArraySize, wireOptionalSize, wireResultSize, wireStringSize } from {{ runtime_package }};
import type { BoltFFIExports, Duration, WireCodec } from {{ runtime_package }};

let _module: BoltFFIModule;
let _exports: BoltFFIExports;
const _callbackImports: Record<string, WebAssembly.ImportValue> = {};

export default async function init(source: BufferSource | Response): Promise<void> {
  _module = await instantiateBoltFFI(source, 1, { env: _callbackImports });
  _exports = _module.exports;
}
