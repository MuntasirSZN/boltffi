from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
from typing import Any

from .contract import ensure_generator


def sha256(path: pathlib.Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def artifact_record(
    path: pathlib.Path, relative_path: str | None = None
) -> dict[str, Any]:
    return {
        "file_name": path.name,
        "relative_path": relative_path,
        "size_bytes": path.stat().st_size,
        "sha256": sha256(path),
    }


def verify_record(path: pathlib.Path, expected: dict[str, Any]) -> None:
    actual = artifact_record(path, expected.get("relative_path"))
    if actual != expected:
        raise SystemExit(f"artifact disagrees with provenance: {path}")


def tree_records(root: pathlib.Path) -> list[dict[str, Any]]:
    paths = sorted(path for path in root.rglob("*") if path.is_file())
    return [artifact_record(path, path.relative_to(root).as_posix()) for path in paths]


def generated_artifact_paths(
    generated: pathlib.Path, generation: dict[str, Any]
) -> dict[str, pathlib.Path]:
    package = generated / "com" / "example" / "bench_boltffi"
    paths = {
        "java_source": package / generation["artifacts"]["java_source"]["file_name"],
        "jni_source": generated
        / "jni"
        / generation["artifacts"]["jni_source"]["file_name"],
        "jni_header": generated
        / "jni"
        / generation["artifacts"]["jni_header"]["file_name"],
        "jni_library": generated / generation["artifacts"]["jni_library"]["file_name"],
    }
    if "java_runtime_source" in generation["artifacts"]:
        paths["java_runtime_source"] = (
            package / generation["artifacts"]["java_runtime_source"]["file_name"]
        )
    return paths


def verify_generation_sources(args: argparse.Namespace) -> dict[str, Any]:
    generator = ensure_generator(args.generator)
    generation = json.loads(args.generation_provenance.read_text())
    if generation.get("generator") != generator:
        raise SystemExit(f"generation provenance does not describe {generator}")
    artifact_files = frozenset(
        path.name for path in args.generation_artifacts.iterdir() if path.is_file()
    )
    expected_artifact_files = frozenset(
        artifact["file_name"] for artifact in generation["artifacts"].values()
    )
    if artifact_files != expected_artifact_files:
        raise SystemExit(
            f"generation artifact bundle mismatch: missing={sorted(expected_artifact_files - artifact_files)}, "
            f"unexpected={sorted(artifact_files - expected_artifact_files)}"
        )
    tuple(
        map(
            lambda item: verify_record(
                args.generation_artifacts / item[1]["file_name"], item[1]
            ),
            generation["artifacts"].items(),
        )
    )
    tuple(
        map(
            lambda item: verify_record(item[1], generation["artifacts"][item[0]]),
            generated_artifact_paths(args.generated, generation).items(),
        )
    )
    return generation


def verify_sources(args: argparse.Namespace) -> None:
    verify_generation_sources(args)
