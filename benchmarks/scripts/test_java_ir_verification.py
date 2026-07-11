from __future__ import annotations

import argparse
import json
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from java_ir.contract import expected_symbols, jni_export
from java_ir.provenance import generated_artifact_paths, verify_generation_sources
from java_ir.symbols import verify_generation


class JavaIrVerificationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        self.expected = expected_symbols("ir")
        self.exports = frozenset(map(jni_export, self.expected))
        self.module = self.root / "BenchBoltFFI.java"
        self.runtime = self.root / "BoltFFINativeRuntime.java"
        self.header = self.root / "demo.h"
        self.jni_source = self.root / "jni_glue.c"
        self.static_library = self.root / "libdemo.a"
        self.jni_library = self.root / "libdemo_jni.dylib"
        self.provenance = self.root / "provenance.json"
        self.runtime.write_text("final class BoltFFINativeRuntime {}")
        self.static_library.write_bytes(b"archive")
        self.jni_library.write_bytes(b"library")
        self.write_surfaces()

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def write_surfaces(
        self,
        header_extra: str = "",
        jni_call_extra: str = "",
        jni_export_extra: str = "",
    ) -> None:
        self.module.write_text(
            "\n".join(f"static native int {symbol}();" for symbol in self.expected)
        )
        self.header.write_text(
            "\n".join([*(f"int {symbol}();" for symbol in self.expected), header_extra])
        )
        self.jni_source.write_text(
            "\n".join(
                [
                    *(
                        f"int call_{symbol}() {{ return {symbol}(); }}"
                        for symbol in self.expected
                    ),
                    *(
                        f"JNIEXPORT void JNICALL {export}(void);"
                        for export in self.exports
                    ),
                    jni_call_extra,
                    jni_export_extra,
                ]
            )
        )

    def arguments(self) -> argparse.Namespace:
        return argparse.Namespace(
            generator="ir",
            module=self.module,
            runtime=self.runtime,
            header=self.header,
            jni_source=self.jni_source,
            static_library=self.static_library,
            jni_library=self.jni_library,
            output=self.provenance,
        )

    def verify(self, library_extra: frozenset[str] = frozenset()) -> None:
        with (
            patch("java_ir.symbols.archive_symbols", return_value=self.expected),
            patch(
                "java_ir.symbols.defined_symbols",
                return_value=self.expected | self.exports | library_extra,
            ),
            patch("java_ir.symbols.symbol_tool", return_value="nm"),
        ):
            verify_generation(self.arguments())

    def test_rejects_header_only_boundary_symbol(self) -> None:
        self.write_surfaces(header_extra="int boltffi_header_only();")
        with self.assertRaises(SystemExit):
            self.verify()

    def test_rejects_jni_only_boundary_symbol(self) -> None:
        self.write_surfaces(jni_call_extra="int x() { return boltffi_jni_only(); }")
        with self.assertRaises(SystemExit):
            self.verify()

    def test_rejects_additional_jni_export(self) -> None:
        extra = "Java_com_example_bench_1boltffi_Native_boltffi_1extra"
        self.write_surfaces(jni_export_extra=f"JNIEXPORT void JNICALL {extra}(void);")
        with self.assertRaises(SystemExit):
            self.verify(frozenset({extra}))

    def test_rejects_generated_source_mutation_before_compilation(self) -> None:
        self.verify()
        generation_artifacts = self.root / "generation-artifacts"
        generated = self.root / "generated"
        generation_artifacts.mkdir()
        generated.mkdir()
        generation = json.loads(self.provenance.read_text())
        sources = {
            "java_source": self.module,
            "java_runtime_source": self.runtime,
            "jni_source": self.jni_source,
            "jni_header": self.header,
            "rust_static_library": self.static_library,
            "jni_library": self.jni_library,
        }
        tuple(
            map(
                lambda item: shutil.copy2(
                    sources[item[0]], generation_artifacts / item[1]["file_name"]
                ),
                generation["artifacts"].items(),
            )
        )
        generated_paths = generated_artifact_paths(generated, generation)
        tuple(
            map(
                lambda path: path.parent.mkdir(parents=True, exist_ok=True),
                generated_paths.values(),
            )
        )
        tuple(
            map(
                lambda item: shutil.copy2(sources[item[0]], item[1]),
                generated_paths.items(),
            )
        )
        generated_paths["java_source"].write_text("mutated")
        arguments = argparse.Namespace(
            generator="ir",
            generated=generated,
            generation_artifacts=generation_artifacts,
            generation_provenance=self.provenance,
        )

        with self.assertRaises(SystemExit):
            verify_generation_sources(arguments)


if __name__ == "__main__":
    unittest.main()
