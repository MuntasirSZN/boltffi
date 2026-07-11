from __future__ import annotations

import argparse
import json
import math
from typing import Any

from .contract import (
    NON_INFERIORITY_MARGIN,
    PAIRS,
    RUNS,
    benchmark_case,
    benchmark_suite,
)
from .preparation import verify_prepared
from .provenance import artifact_record
from .results import merge_generator_results, validate_loaded_library, validate_result
from .statistics import infer


def compare(args: argparse.Namespace) -> None:
    suite = benchmark_suite(args.suite)
    results_root = args.results.resolve()
    prepared_roots = {
        generator: args.prepared / generator for generator in ("legacy", "ir")
    }
    prepared = {
        generator: json.loads((root / "prepared-provenance.json").read_text())
        for generator, root in prepared_roots.items()
    }
    invalid_prepared = tuple(
        generator
        for generator, document in prepared.items()
        if document.get("generator") != generator
        or document.get("suite") != suite.name
        or document.get("abi") != ("legacy" if generator == "legacy" else "binding-ir")
        or document.get("generation", {}).get("generator") != generator
        or document.get("generation", {}).get("abi")
        != ("legacy" if generator == "legacy" else "binding-ir")
    )
    if invalid_prepared:
        raise SystemExit(
            "prepared provenance has the wrong generator identity: "
            + ", ".join(invalid_prepared)
        )
    tuple(
        map(
            lambda generator: verify_prepared(
                prepared_roots[generator], prepared[generator]
            ),
            ("legacy", "ir"),
        )
    )
    loaded = {
        run.name: validate_result(
            run,
            args.runs / f"{run.name}.json",
            tuple(prepared_roots.values()),
            suite,
        )
        for run in RUNS
    }
    identities = frozenset(result[2] for result in loaded.values())
    if len(identities) != 1:
        raise SystemExit("JMH configuration differed across the twelve ABBA runs")
    measured_launcher = next(iter(identities))[0]
    prepared_launchers = frozenset(
        document["java_launcher"] for document in prepared.values()
    )
    if prepared_launchers != {measured_launcher}:
        raise SystemExit("JMH did not use the prepared Java launcher")
    loaded_paths = {
        run.name: validate_loaded_library(
            run,
            args.runs / f"{run.name}.log",
            prepared_roots[run.generator],
            prepared[run.generator],
            suite,
        )
        for run in RUNS
    }

    def case_scores(run_name: str) -> dict[str, float]:
        return {
            benchmark_case(suite, benchmark): float(entry["primaryMetric"]["score"])
            for benchmark, entry in loaded[run_name][1].items()
        }

    scores = {run.name: case_scores(run.name) for run in RUNS}
    inference = infer(scores, suite.cases)
    legacy = merge_generator_results("legacy", loaded)
    binding_ir = merge_generator_results("ir", loaded)
    args.results.mkdir(parents=True, exist_ok=True)
    (args.results / "legacy-results.json").write_text(
        json.dumps(legacy, indent=2) + "\n"
    )
    (args.results / "ir-results.json").write_text(
        json.dumps(binding_ir, indent=2) + "\n"
    )
    (args.results / "results.json").write_text(
        json.dumps(legacy + binding_ir, indent=2) + "\n"
    )
    run_artifacts = {
        run.name: {
            "generator": run.generator,
            "native_library": {
                "path": loaded_paths[run.name].relative_to(results_root).as_posix(),
                "sha256": prepared[run.generator]["native_library"]["sha256"],
            },
            "load_log": artifact_record(
                args.runs / f"{run.name}.log", f"runs/{run.name}.log"
            ),
            "raw_result": artifact_record(
                args.runs / f"{run.name}.json", f"runs/{run.name}.json"
            ),
        }
        for run in RUNS
    }
    comparison: dict[str, Any] = {
        "design": {
            "suite": suite.name,
            "order": [run.generator for run in RUNS],
            "cycles": 3,
            "forks_per_position": 1,
            "warmup_iterations": 3,
            "warmup_time": "1 s",
            "measurement_iterations": 3,
            "measurement_time": "1 s",
            "threads": 1,
            "mode": "avgt",
            "unit": "ns/op",
        },
        "non_inferiority": {
            "margin": NON_INFERIORITY_MARGIN,
            "confidence": 0.95,
            "distribution": "student_t",
            "degrees_of_freedom": 5,
            "established": not inference.inconclusive,
            "inconclusive_cases": list(inference.inconclusive),
            "paired_log_ratios": {
                case: list(values) for case, values in inference.log_ratios.items()
            },
            "point_ratios": inference.point_ratios,
            "upper_ratio_bounds": inference.upper_bounds,
        },
        "pairs": [
            {
                "cycle": pair.cycle,
                "position": pair.position,
                "legacy": pair.legacy_run,
                "ir": pair.ir_run,
            }
            for pair in PAIRS
        ],
        "runs": run_artifacts,
        "prepared": prepared,
    }
    (args.results / "comparison-provenance.json").write_text(
        json.dumps(comparison, indent=2) + "\n"
    )
    cases = tuple(case.name for case in suite.cases)
    print("case                 ratio       one-sided 95% upper")
    print(
        "\n".join(
            f"{case:18} {inference.point_ratios[case]:10.4f} "
            f"{inference.upper_bounds[case]:24.4f}"
            for case in sorted(cases)
        )
    )


def enforce_verdict(args: argparse.Namespace) -> None:
    comparison = json.loads(args.provenance.read_text())
    suite = benchmark_suite(comparison.get("design", {}).get("suite", ""))
    non_inferiority = comparison.get("non_inferiority")
    if not isinstance(non_inferiority, dict):
        raise SystemExit("comparison provenance has no non-inferiority result")
    expected_cases = frozenset(case.name for case in suite.cases)
    upper_bounds = non_inferiority.get("upper_ratio_bounds")
    try:
        margin = float(non_inferiority["margin"])
        parsed_bounds = {case: float(bound) for case, bound in upper_bounds.items()}
    except (AttributeError, KeyError, TypeError, ValueError) as failure:
        raise SystemExit("comparison provenance has a malformed verdict") from failure
    if (
        frozenset(parsed_bounds) != expected_cases
        or not math.isfinite(margin)
        or margin != NON_INFERIORITY_MARGIN
        or any(
            not math.isfinite(bound) or bound <= 0 for bound in parsed_bounds.values()
        )
    ):
        raise SystemExit("comparison provenance has a malformed verdict")
    derived_inconclusive = tuple(
        case for case in sorted(expected_cases) if parsed_bounds[case] > margin
    )
    recorded_inconclusive = tuple(non_inferiority.get("inconclusive_cases", ()))
    established = non_inferiority.get("established")
    if recorded_inconclusive != derived_inconclusive or established is not (
        not derived_inconclusive
    ):
        raise SystemExit("comparison provenance has an inconsistent verdict")
    if derived_inconclusive:
        raise SystemExit(
            f"Binding IR did not establish the {margin:.2f} non-inferiority margin for: "
            + ", ".join(derived_inconclusive)
        )
