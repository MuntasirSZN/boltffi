import { BoltFFIModule, instantiateBoltFFISync, wireOptionalSize, wireStringSize } from {{ runtime_package }};
import type { BoltFFIExports } from {{ runtime_package }};
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const _thisDir = dirname(fileURLToPath(import.meta.url));
const _wasmPath = join(_thisDir, {{ wasm_file }});
const _callbackImports: Record<string, WebAssembly.ImportValue> = {};
