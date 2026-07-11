from __future__ import annotations

import copy
import json
import math
import pathlib
import re
import statistics
from typing import Any, Sequence

from .contract import (
    EXPECTED_JVM_ARGS,
    RUNS,
    TWO_SIDED_T_95_DF_5,
    BenchmarkSuite,
    ComparisonRun,
    benchmark_case,
)
from .provenance import verify_record


LOADED_LIBRARY_PATTERN = re.compile(r"Loaded library (.+?), handle\b")


def normalized_jvm_args(
    entry: dict[str, Any], prepared_roots: Sequence[pathlib.Path]
) -> tuple[str, ...]:
    replacements = tuple(str(root / "generated") for root in prepared_roots)

    def normalize(argument: str) -> str:
        return next(
            (
                argument.replace(replacement, "<prepared-java>")
                for replacement in replacements
                if replacement in argument
            ),
            argument,
        )

    return tuple(map(normalize, entry.get("jvmArgs", [])))


def validate_result(
    run: ComparisonRun,
    path: pathlib.Path,
    prepared_roots: Sequence[pathlib.Path],
    suite: BenchmarkSuite,
) -> tuple[list[dict[str, Any]], dict[str, dict[str, Any]], tuple[Any, ...]]:
    entries = json.loads(path.read_text())
    expected_cases = frozenset(case.name for case in suite.cases)
    if len(entries) != len(expected_cases):
        raise SystemExit(
            f"{run.name} emitted {len(entries)} {suite.name} benchmarks"
        )
    indexed = {entry["benchmark"]: entry for entry in entries}
    if len(indexed) != len(entries):
        raise SystemExit(f"{run.name} emitted duplicate benchmark identifiers")
    actual_cases = frozenset(benchmark_case(suite, benchmark) for benchmark in indexed)
    if actual_cases != expected_cases:
        raise SystemExit(
            f"{run.name} {suite.name} contract mismatch: "
            f"missing={sorted(expected_cases - actual_cases)}, "
            f"unexpected={sorted(actual_cases - expected_cases)}"
        )

    def validate_entry(entry: dict[str, Any]) -> None:
        primary = entry["primaryMetric"]
        expected_shape = (
            entry.get("jmhVersion") == "1.37"
            and entry.get("mode") == "avgt"
            and entry.get("threads") == 1
            and entry.get("forks") == 1
            and entry.get("warmupIterations") == 3
            and entry.get("warmupTime") == "1 s"
            and entry.get("warmupBatchSize") == 1
            and entry.get("measurementIterations") == 3
            and entry.get("measurementTime") == "1 s"
            and entry.get("measurementBatchSize") == 1
            and primary.get("scoreUnit") == "ns/op"
            and normalized_jvm_args(entry, prepared_roots) == EXPECTED_JVM_ARGS
            and entry.get("secondaryMetrics") in ({}, None)
            and len(primary.get("rawData", [])) == 1
            and len(primary.get("rawData", [[]])[0]) == 3
        )
        if not expected_shape:
            raise SystemExit(f"invalid JMH configuration or sample shape in {run.name}")
        samples = tuple(map(float, primary["rawData"][0]))
        score = float(primary["score"])
        if (
            not math.isfinite(score)
            or score <= 0
            or not all(map(math.isfinite, samples))
            or any(map(lambda sample: sample <= 0, samples))
        ):
            raise SystemExit(f"invalid JMH samples in {run.name}")

    tuple(map(validate_entry, entries))
    first = entries[0]

    def identity(entry: dict[str, Any]) -> tuple[Any, ...]:
        return (
            entry.get("jvm"),
            normalized_jvm_args(entry, prepared_roots),
            entry.get("jdkVersion"),
            entry.get("vmName"),
            entry.get("vmVersion"),
        )

    run_identity = identity(first)
    if any(identity(entry) != run_identity for entry in entries[1:]):
        raise SystemExit(f"JMH configuration varied within {run.name}")
    return entries, indexed, run_identity


def validate_loaded_library(
    run: ComparisonRun,
    log_path: pathlib.Path,
    prepared_root: pathlib.Path,
    prepared: dict[str, Any],
    suite: BenchmarkSuite,
) -> pathlib.Path:
    expected = prepared_root / prepared["native_library"]["relative_path"]
    verify_record(expected, prepared["native_library"])
    log = log_path.read_text(errors="replace")
    partitioned = tuple(log.split("# Fork: 1 of 1"))
    sections = partitioned[1:]
    if len(sections) != len(suite.cases):
        raise SystemExit(
            f"{run.name} logged {len(sections)} JMH forks, expected {len(suite.cases)}"
        )

    def is_demo_library(path: pathlib.Path) -> bool:
        return path.name == expected.name or bool(
            re.fullmatch(
                r"(?:lib)?demo(?:_jni)?\.(?:dll|dylib|jnilib|so(?:\.[0-9.]+)?)",
                path.name,
            )
        )

    def loaded_candidates(section: str) -> tuple[pathlib.Path, ...]:
        return tuple(
            path
            for path in map(pathlib.Path, LOADED_LIBRARY_PATTERN.findall(section))
            if is_demo_library(path)
        )

    if loaded_candidates(partitioned[0]):
        raise SystemExit(f"{run.name} loaded a JNI library outside a benchmark fork")
    candidates = tuple(map(loaded_candidates, sections))
    invalid = tuple(
        index
        for index, fork_candidates in enumerate(candidates, 1)
        if len(fork_candidates) != 1
        or fork_candidates[0].resolve() != expected.resolve()
    )
    if invalid:
        raise SystemExit(f"{run.name} loaded the wrong JNI library in forks: {invalid}")
    return expected.resolve()


def percentile(samples: Sequence[float], percentage: float) -> float:
    ordered = sorted(samples)
    position = (len(ordered) - 1) * percentage / 100
    lower_index = math.floor(position)
    upper_index = math.ceil(position)
    if lower_index == upper_index:
        return ordered[lower_index]
    lower_weight = upper_index - position
    return ordered[lower_index] * lower_weight + ordered[upper_index] * (
        1 - lower_weight
    )


def merge_generator_results(
    generator: str,
    loaded: dict[
        str, tuple[list[dict[str, Any]], dict[str, dict[str, Any]], tuple[Any, ...]]
    ],
) -> list[dict[str, Any]]:
    run_names = tuple(run.name for run in RUNS if run.generator == generator)
    benchmark_order = tuple(entry["benchmark"] for entry in loaded[run_names[0]][0])

    def merge_benchmark(benchmark_name: str) -> dict[str, Any]:
        entries = tuple(loaded[run_name][1][benchmark_name] for run_name in run_names)
        run_scores = tuple(float(entry["primaryMetric"]["score"]) for entry in entries)
        raw_data = [entry["primaryMetric"]["rawData"][0] for entry in entries]
        samples = tuple(float(sample) for group in raw_data for sample in group)
        score = statistics.fmean(run_scores)
        error = (
            TWO_SIDED_T_95_DF_5
            * statistics.stdev(run_scores)
            / math.sqrt(len(run_scores))
        )
        metric = copy.deepcopy(entries[0]["primaryMetric"])
        metric.update(
            {
                "score": score,
                "scoreError": error,
                "scoreConfidence": [score - error, score + error],
                "scorePercentiles": {
                    key: percentile(samples, float(key))
                    for key in metric.get("scorePercentiles", {})
                },
                "rawData": raw_data,
            }
        )
        merged = copy.deepcopy(entries[0])
        merged["forks"] = len(entries)
        merged["primaryMetric"] = metric
        if generator == "ir":
            merged["benchmark"] = benchmark_name.replace(
                ".boltffi_java_", ".boltffi_java_ir_", 1
            )
        return merged

    return list(map(merge_benchmark, benchmark_order))
