# C example

This directory calls the shared Rust demo library through its generated C API. Start with [example.c](example.c): it passes a string and a record, uses a class, checks a result, and frees the returned owners.

The C target is experimental. It supports synchronous functions and methods, records, enums with payloads, collections, custom types, and constants. See the [C guide](https://boltffi.dev/docs/c) for ownership, callbacks, and current limitations.

## Run

The build needs Rust, a C11 compiler, and CMake 3.20 or later. From the repository root:

```sh
bash examples/platforms/c/test-demo.sh
```

The script builds the Rust library with the `c-demo` feature, generates `generated/include/demo.h`, packages the native libraries under `generated/lib`, and runs both the small example and the test suite. The feature includes extra records used to exercise mutable C values.

The same build runs in CI on Linux, macOS, and Windows. On Windows, use an environment with the MSVC toolchain available. The cross-platform demo runner can also select C:

```sh
bash examples/demo/verify-platform-demos.sh --platform c
```

After the script finishes, run the example again with:

```sh
./examples/platforms/c/build/c_example
```

With a multi-configuration generator such as Visual Studio, the executable is under `build/Debug/c_example.exe`.

## Use the package elsewhere

Include `demo.h`, add `generated/include` to the compiler's include path, and link the shared library from `generated/lib`. CMake copies the DLL beside the executable on Windows; on Unix it sets up the build's runtime library search path.

For a separate project, the generated artifacts are enough. There is no dependency on this CMake project. The [linking guide](https://boltffi.dev/docs/c#link-a-c-program) gives compiler commands and explains static versus shared linking.

## Further examples

The [tests](tests) contain executable examples of strings and byte buffers, optional values, direct and owning records, enums, nested collections and maps, custom types, results, constants, class handles, and callback ownership. Each file covers one API category. Async functions, streams, and callbacks needing unsupported conversions are recorded as gaps in the demo coverage audit, not represented as passing tests.
