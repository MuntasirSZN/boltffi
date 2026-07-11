from __future__ import annotations

import argparse
import json
import pathlib
import zipfile
from typing import Any

from .contract import benchmark_suite, ensure_generator
from .provenance import (
    artifact_record,
    generated_artifact_paths,
    tree_records,
    verify_generation_sources,
    verify_record,
)


JAVA_CLASS_ENTRY = "com/example/bench_boltffi/BenchBoltFFI.class"
RUNTIME_CLASS_ENTRY = "com/example/bench_boltffi/BoltFFINativeRuntime.class"


def prepare(args: argparse.Namespace) -> None:
    generation = verify_generation_sources(args)
    generator = ensure_generator(args.generator)
    suite = benchmark_suite(args.suite)
    benchmark_class_entry = (
        f"com/example/bench_compare/{suite.benchmark_class}.class"
    )
    with zipfile.ZipFile(args.jar) as archive:
        names = frozenset(archive.namelist())
        required_entries = {JAVA_CLASS_ENTRY, benchmark_class_entry}
        if "java_runtime_source" in generation["artifacts"]:
            required_entries.add(RUNTIME_CLASS_ENTRY)
        missing_entries = sorted(required_entries - names)
        if missing_entries:
            raise SystemExit(f"JMH jar is missing executed classes: {missing_entries}")
        if archive.read(JAVA_CLASS_ENTRY) != args.java_class.read_bytes():
            raise SystemExit(
                "compiled BenchBoltFFI.class does not match the runnable JMH jar"
            )
    native_library = generated_artifact_paths(args.generated, generation)["jni_library"]
    verify_record(native_library, generation["artifacts"]["jni_library"])
    java_launcher = pathlib.Path(args.java_launcher.read_text().strip()).resolve(
        strict=True
    )
    prepared = {
        "generator": generator,
        "abi": generation["abi"],
        "suite": suite.name,
        "java_launcher": str(java_launcher),
        "native_library": artifact_record(
            native_library, native_library.relative_to(args.root).as_posix()
        ),
        "java_class": artifact_record(
            args.java_class, args.java_class.relative_to(args.root).as_posix()
        ),
        "jmh_jar": artifact_record(
            args.jar, args.jar.relative_to(args.root).as_posix()
        ),
        "generated_files": tree_records(args.generated),
        "generation_artifacts": tree_records(args.generation_artifacts),
        "generation": generation,
    }
    args.output.write_text(json.dumps(prepared, indent=2) + "\n")


def verify_prepared(root: pathlib.Path, prepared: dict[str, Any]) -> None:
    tuple(
        map(
            lambda key: verify_record(
                root / prepared[key]["relative_path"], prepared[key]
            ),
            ("native_library", "java_class", "jmh_jar"),
        )
    )
    if tree_records(root / "generated") != prepared["generated_files"]:
        raise SystemExit(f"prepared generated tree changed: {root}")
    if tree_records(root / "generation-artifacts") != prepared["generation_artifacts"]:
        raise SystemExit(f"prepared generation artifacts changed: {root}")
    pathlib.Path(prepared["java_launcher"]).resolve(strict=True)
