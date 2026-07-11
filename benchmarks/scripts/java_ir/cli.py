from __future__ import annotations

import argparse
import pathlib

from .comparison import compare, enforce_verdict
from .preparation import prepare
from .provenance import verify_sources
from .symbols import verify_generation


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser()
    commands = root.add_subparsers(dest="command", required=True)
    verification = commands.add_parser("verify-generation")
    verification.add_argument("--generator", required=True)
    verification.add_argument("--module", type=pathlib.Path, required=True)
    verification.add_argument("--runtime", type=pathlib.Path)
    verification.add_argument("--header", type=pathlib.Path, required=True)
    verification.add_argument("--jni-source", type=pathlib.Path, required=True)
    verification.add_argument("--static-library", type=pathlib.Path, required=True)
    verification.add_argument("--jni-library", type=pathlib.Path, required=True)
    verification.add_argument("--output", type=pathlib.Path, required=True)
    verification.set_defaults(operation=verify_generation)
    source_verification = commands.add_parser("verify-sources")
    source_verification.add_argument("--generator", required=True)
    source_verification.add_argument("--generated", type=pathlib.Path, required=True)
    source_verification.add_argument(
        "--generation-artifacts", type=pathlib.Path, required=True
    )
    source_verification.add_argument(
        "--generation-provenance", type=pathlib.Path, required=True
    )
    source_verification.set_defaults(operation=verify_sources)
    preparation = commands.add_parser("prepare")
    preparation.add_argument("--generator", required=True)
    preparation.add_argument("--suite", required=True)
    preparation.add_argument("--root", type=pathlib.Path, required=True)
    preparation.add_argument("--generated", type=pathlib.Path, required=True)
    preparation.add_argument("--generation-artifacts", type=pathlib.Path, required=True)
    preparation.add_argument(
        "--generation-provenance", type=pathlib.Path, required=True
    )
    preparation.add_argument("--java-class", type=pathlib.Path, required=True)
    preparation.add_argument("--jar", type=pathlib.Path, required=True)
    preparation.add_argument("--java-launcher", type=pathlib.Path, required=True)
    preparation.add_argument("--output", type=pathlib.Path, required=True)
    preparation.set_defaults(operation=prepare)
    comparison = commands.add_parser("compare")
    comparison.add_argument("--suite", required=True)
    comparison.add_argument("--prepared", type=pathlib.Path, required=True)
    comparison.add_argument("--runs", type=pathlib.Path, required=True)
    comparison.add_argument("--results", type=pathlib.Path, required=True)
    comparison.set_defaults(operation=compare)
    verdict = commands.add_parser("verdict")
    verdict.add_argument("--provenance", type=pathlib.Path, required=True)
    verdict.set_defaults(operation=enforce_verdict)
    return root


def main() -> None:
    args = parser().parse_args()
    args.operation(args)
