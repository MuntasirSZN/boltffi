from __future__ import annotations

import argparse
import json
import pathlib
import re
from dataclasses import dataclass
from typing import Any

from .contract import benchmark_case, benchmark_suite


@dataclass(frozen=True)
class Baseline:
    document: dict[str, Any]

    @classmethod
    def load(
        cls,
        provenance: dict[str, Any],
        results: list[dict[str, Any]],
        revision: str,
    ) -> Baseline:
        if re.fullmatch(r"[0-9a-f]{7,40}", revision) is None:
            raise SystemExit("baseline revision must be a Git object name")
        suite = benchmark_suite(provenance.get("design", {}).get("suite", ""))
        expected_cases = frozenset(case.name for case in suite.cases)
        scores = {"legacy": {}, "ir": {}}

        def record(entry: dict[str, Any]) -> None:
            benchmark = entry.get("benchmark", "")
            ir_marker = ".boltffi_java_ir_"
            generator = "ir" if ir_marker in benchmark else "legacy"
            normalized = benchmark.replace(ir_marker, ".boltffi_java_", 1)
            case = benchmark_case(suite, normalized)
            score = float(entry["primaryMetric"]["score"])
            if case in scores[generator]:
                raise SystemExit(f"duplicate {generator} baseline case: {case}")
            scores[generator][case] = score

        tuple(map(record, results))
        if any(
            frozenset(generator_scores) != expected_cases
            for generator_scores in scores.values()
        ):
            raise SystemExit("baseline results do not match the comparison suite")
        prepared = provenance.get("prepared", {})
        non_inferiority = provenance.get("non_inferiority")
        runs = provenance.get("runs", {})
        if frozenset(prepared) != {"legacy", "ir"} or not isinstance(non_inferiority, dict):
            raise SystemExit("comparison provenance is incomplete")
        first = results[0]
        return cls(
            {
                "schema": 1,
                "revision": revision,
                "suite": suite.name,
                "environment": {
                    "jdk": first.get("jdkVersion"),
                    "vm": first.get("vmName"),
                    "vm_version": first.get("vmVersion"),
                    "mode": first.get("mode"),
                    "unit": first.get("primaryMetric", {}).get("scoreUnit"),
                },
                "design": provenance["design"],
                "artifacts": {
                    generator: {
                        "native_library": prepared[generator]["native_library"]["sha256"],
                        "java_class": prepared[generator]["java_class"]["sha256"],
                    }
                    for generator in ("legacy", "ir")
                },
                "run_results": {
                    name: run["raw_result"]["sha256"] for name, run in runs.items()
                },
                "scores_ns": scores,
                "non_inferiority": non_inferiority,
            }
        )

    def write(self, path: pathlib.Path) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(self.document, indent=2) + "\n")


def archive(args: argparse.Namespace) -> None:
    Baseline.load(
        json.loads(args.provenance.read_text()),
        json.loads(args.results.read_text()),
        args.revision,
    ).write(args.output)
