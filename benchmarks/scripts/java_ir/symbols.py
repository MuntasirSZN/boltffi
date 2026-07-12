from __future__ import annotations

import argparse
import json
import os
import pathlib
import platform
import re
import shutil
import subprocess
import tempfile
from typing import Iterable

from .contract import (
    ALL_CASES,
    ensure_generator,
    expected_symbols,
    jni_export,
    opposite_symbols,
)
from .provenance import artifact_record


HEADER_SUPPORT_SYMBOLS = frozenset(
    {
        "boltffi_atomic_u64_cas",
        "boltffi_atomic_u64_exchange",
        "boltffi_atomic_u64_load",
        "boltffi_atomic_u8_cas",
        "boltffi_buf_from_bytes",
        "boltffi_buf_with_len",
        "boltffi_clear_last_error",
        "boltffi_free_buf",
        "boltffi_free_string",
        "boltffi_last_error_message",
    }
)
JNI_SUPPORT_SYMBOLS = frozenset(
    {
        "boltffi_jni_throw_illegal_argument",
        "boltffi_jni_throw_runtime",
        "boltffi_jni_throw_status",
    }
)
BOUNDARY_PATTERN = re.compile(r"\b(boltffi_[A-Za-z0-9_]+)\s*\(")
JAVA_NATIVE_PATTERN = re.compile(r"\bnative\b[^;{}]*?\b(boltffi_[A-Za-z0-9_]+)\s*\(")
JNI_EXPORT_PATTERN = re.compile(r"\bJNICALL\s+(Java_[A-Za-z0-9_]+)\s*\(")
JNI_STATIC_FUNCTION_PATTERN = re.compile(
    r"\bstatic\b[^;{}]*\b(boltffi_jni_[A-Za-z0-9_]+)\s*\("
)
BRIDGE_REGISTRATION_PREFIXES = (
    "boltffi_create_callback_",
    "boltffi_register_callback_",
)
JAVA_HELPER_PREFIXES = (
    "boltffi_callback_handle_",
    "boltffi_success_",
)
DEFINED_SYMBOL_PATTERN = re.compile(
    r"\b([A-Za-z])\s+_?((?:boltffi_|Java_)[A-Za-z0-9_]+)\b"
)


def symbol_tool() -> str:
    override_name = os.environ.get("BOLTFFI_NM", "")
    override = shutil.which(override_name)
    host = platform.system()
    if override_name and override is None:
        raise SystemExit(
            f"BOLTFFI_NM does not resolve to an executable: {override_name}"
        )
    if override is None and host not in {"Darwin", "FreeBSD", "Linux"}:
        raise SystemExit(
            f"Java benchmark symbol verification is unsupported on {host}: "
            "set BOLTFFI_NM to an llvm-nm-compatible executable"
        )
    candidates = tuple(filter(None, map(shutil.which, ("llvm-nm", "rust-nm", "nm"))))
    tool = override or next(iter(candidates), None)
    if tool is not None:
        return tool
    raise SystemExit(
        f"Java benchmark symbol verification is unsupported on {host}: "
        "set BOLTFFI_NM to an llvm-nm-compatible executable"
    )


def defined_symbols(path: pathlib.Path) -> frozenset[str]:
    completed = subprocess.run(
        (symbol_tool(), "-g", str(path)), capture_output=True, text=True, check=False
    )
    if completed.returncode != 0:
        raise SystemExit(
            f"symbol inspection failed for {path} with exit {completed.returncode}: "
            f"{completed.stderr.strip()}"
        )

    def parse(line: str) -> str | None:
        match = DEFINED_SYMBOL_PATTERN.search(line)
        if match is None or match.group(1).upper() == "U":
            return None
        return match.group(2)

    return frozenset(filter(None, map(parse, completed.stdout.splitlines())))


def archive_symbols(path: pathlib.Path) -> frozenset[str]:
    archive_tool = shutil.which("ar")
    if archive_tool is None:
        return defined_symbols(path)
    listing = subprocess.run(
        (archive_tool, "-t", str(path.resolve())),
        capture_output=True,
        text=True,
        check=False,
    )
    if listing.returncode != 0:
        raise SystemExit(
            f"archive inspection failed for {path} with exit {listing.returncode}: "
            f"{listing.stderr.strip()}"
        )
    members = tuple(
        member
        for member in listing.stdout.splitlines()
        if member.startswith("demo.") and member.endswith((".o", ".obj"))
    )
    if not members:
        return defined_symbols(path)
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        extraction = subprocess.run(
            (archive_tool, "-x", str(path.resolve()), *members),
            cwd=root,
            capture_output=True,
            text=True,
            check=False,
        )
        if extraction.returncode != 0:
            raise SystemExit(
                f"archive extraction failed for {path} with exit {extraction.returncode}: "
                f"{extraction.stderr.strip()}"
            )
        return frozenset().union(*map(defined_symbols, map(root.joinpath, members)))


def require_symbols(
    surface: str, actual: Iterable[str], expected: frozenset[str]
) -> None:
    missing = sorted(expected - frozenset(actual))
    if missing:
        raise SystemExit(f"{surface} is missing expected symbols: {', '.join(missing)}")


def reject_symbols(
    surface: str, actual: Iterable[str], forbidden: frozenset[str]
) -> None:
    present = sorted(frozenset(actual) & forbidden)
    if present:
        raise SystemExit(
            f"{surface} leaked opposite-generator symbols: {', '.join(present)}"
        )


def require_exact_symbols(
    surface: str, actual: Iterable[str], expected: frozenset[str]
) -> None:
    actual_set = frozenset(actual)
    missing = sorted(expected - actual_set)
    unexpected = sorted(actual_set - expected)
    if missing or unexpected:
        raise SystemExit(
            f"{surface} ABI mismatch: missing={missing}, unexpected={unexpected}"
        )


def verify_generation(args: argparse.Namespace) -> None:
    generator = ensure_generator(args.generator)
    expected = expected_symbols(generator)
    forbidden = opposite_symbols(generator)
    expected_exports = frozenset(map(jni_export, expected))
    forbidden_exports = frozenset(map(jni_export, forbidden))
    module_text = args.module.read_text()
    header_text = args.header.read_text()
    jni_text = args.jni_source.read_text()
    java_native_symbols = frozenset(JAVA_NATIVE_PATTERN.findall(module_text))
    header_symbols = frozenset(BOUNDARY_PATTERN.findall(header_text))
    jni_boundary_symbols = frozenset(BOUNDARY_PATTERN.findall(jni_text))
    jni_static_functions = frozenset(JNI_STATIC_FUNCTION_PATTERN.findall(jni_text))
    header_boundary_symbols = header_symbols - HEADER_SUPPORT_SYMBOLS
    jni_call_symbols = frozenset(
        symbol
        for symbol in jni_boundary_symbols
        if symbol not in JNI_SUPPORT_SYMBOLS
        and symbol not in HEADER_SUPPORT_SYMBOLS
        and symbol not in jni_static_functions
    )
    boundary_symbols = header_boundary_symbols & jni_call_symbols
    jni_source_exports = frozenset(JNI_EXPORT_PATTERN.findall(jni_text))
    archive_symbol_set = archive_symbols(args.static_library)
    library_symbols = defined_symbols(args.jni_library)
    library_exports = frozenset(
        symbol for symbol in library_symbols if symbol.startswith("Java_")
    )
    required_surfaces = (
        ("Java source", java_native_symbols),
        ("header/JNI boundary", boundary_symbols),
        ("native archive", archive_symbol_set),
        ("final JNI library", library_symbols),
    )
    forbidden_surfaces = (
        ("Java source", java_native_symbols),
        ("JNI header", header_symbols),
        ("JNI source", jni_boundary_symbols),
        ("native archive", archive_symbol_set),
        ("final JNI library", library_symbols),
    )
    tuple(
        map(lambda item: require_symbols(item[0], item[1], expected), required_surfaces)
    )
    tuple(
        map(
            lambda item: reject_symbols(item[0], item[1], forbidden), forbidden_surfaces
        )
    )
    require_symbols("final JNI exports", library_exports, jni_source_exports)
    require_symbols("expected JNI source exports", jni_source_exports, expected_exports)
    require_symbols("expected final JNI exports", library_exports, expected_exports)
    reject_symbols("JNI source exports", jni_source_exports, forbidden_exports)
    reject_symbols("final JNI exports", library_exports, forbidden_exports)
    if generator == "ir":
        all_exports = frozenset(map(jni_export, java_native_symbols))
        direct_header_symbols = frozenset(
            symbol
            for symbol in header_boundary_symbols
            if not symbol.startswith(BRIDGE_REGISTRATION_PREFIXES)
        )
        direct_jni_call_symbols = frozenset(
            symbol
            for symbol in jni_call_symbols
            if not symbol.startswith(BRIDGE_REGISTRATION_PREFIXES)
        )
        direct_java_native_symbols = frozenset(
            symbol
            for symbol in java_native_symbols
            if not symbol.startswith(JAVA_HELPER_PREFIXES)
        )
        require_exact_symbols(
            "IR JNI header", direct_header_symbols, direct_java_native_symbols
        )
        require_exact_symbols(
            "IR JNI calls", direct_jni_call_symbols, direct_java_native_symbols
        )
        require_exact_symbols(
            "IR JNI source exports", jni_source_exports, all_exports
        )
        require_exact_symbols("IR final JNI exports", library_exports, all_exports)
    artifacts = {
        "java_source": artifact_record(args.module),
        "jni_source": artifact_record(args.jni_source),
        "jni_header": artifact_record(args.header),
        "rust_static_library": artifact_record(args.static_library),
        "jni_library": artifact_record(args.jni_library),
    }
    if args.runtime is not None:
        artifacts["java_runtime_source"] = artifact_record(args.runtime)
    args.output.write_text(
        json.dumps(
            {
                "generator": generator,
                "abi": "legacy" if generator == "legacy" else "binding-ir",
                "benchmark_cases": [case.name for case in ALL_CASES],
                "expected_boundary_symbols": sorted(expected),
                "verified_boundary_symbols": sorted(boundary_symbols),
                "verified_jni_exports": sorted(jni_source_exports),
                "symbol_tool": symbol_tool(),
                "artifacts": artifacts,
            },
            indent=2,
        )
        + "\n"
    )
