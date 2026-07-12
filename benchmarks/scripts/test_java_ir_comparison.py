from __future__ import annotations

import argparse
import json
import math
import tempfile
import unittest
from pathlib import Path

from java_ir import (
    BENCHMARK_PREFIX,
    PRIMITIVE_CASES,
    SUITES,
    PAIRS,
    RUNS,
    ComparisonRun,
    paired_log_upper,
    reject_symbols,
    require_exact_symbols,
    validate_loaded_library,
    validate_result,
)
from java_ir.provenance import artifact_record
from java_ir.baseline import Baseline
from java_ir.comparison import enforce_verdict


class JavaIrComparisonTests(unittest.TestCase):
    def test_compacts_comparison_into_a_portable_baseline(self) -> None:
        suite = SUITES["primitive"]
        cases = tuple(case.name for case in suite.cases)
        results = [
            {
                "benchmark": suite.prefix + case,
                "jdkVersion": "25",
                "vmName": "OpenJDK",
                "vmVersion": "25",
                "mode": "avgt",
                "primaryMetric": {"score": 1.0, "scoreUnit": "ns/op"},
            }
            for case in cases
        ] + [
            {
                "benchmark": suite.prefix.replace(
                    ".boltffi_java_", ".boltffi_java_ir_"
                )
                + case,
                "jdkVersion": "25",
                "vmName": "OpenJDK",
                "vmVersion": "25",
                "mode": "avgt",
                "primaryMetric": {"score": 0.9, "scoreUnit": "ns/op"},
            }
            for case in cases
        ]
        provenance = {
            "design": {"suite": "primitive"},
            "prepared": {
                generator: {
                    "native_library": {"sha256": generator + "-native"},
                    "java_class": {"sha256": generator + "-java"},
                }
                for generator in ("legacy", "ir")
            },
            "runs": {
                "cycle-1-legacy-a": {"raw_result": {"sha256": "run"}}
            },
            "non_inferiority": {"point_ratios": {case: 0.9 for case in cases}},
        }

        baseline = Baseline.load(provenance, results, "50ee1836")

        self.assertEqual("50ee1836", baseline.document["revision"])
        self.assertEqual(0.9, baseline.document["scores_ns"]["ir"]["echo_bool"])
        self.assertNotIn("prepared", baseline.document)

    def test_uses_three_complete_abba_cycles(self) -> None:
        self.assertEqual(
            ["legacy", "ir", "ir", "legacy"] * 3,
            [run.generator for run in RUNS],
        )
        self.assertEqual(6, len(PAIRS))

    def test_constant_ratio_has_that_upper_bound(self) -> None:
        self.assertAlmostEqual(1.02, paired_log_upper([math.log(1.02)] * 6))

    def test_between_pair_variation_widens_upper_bound(self) -> None:
        centered = [math.log(ratio) for ratio in (0.9, 1.1, 0.9, 1.1, 0.9, 1.1)]
        self.assertGreater(paired_log_upper(centered), 1.05)

    def test_exact_ir_boundary_rejects_extra_legacy_symbol(self) -> None:
        with self.assertRaises(SystemExit):
            require_exact_symbols("IR boundary", {"ir", "legacy"}, frozenset({"ir"}))

    def test_opposite_generator_symbol_is_rejected(self) -> None:
        with self.assertRaises(SystemExit):
            reject_symbols("IR archive", {"ir", "legacy"}, frozenset({"legacy"}))

    def test_requires_exact_measurement_shape(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            prepared_roots = (root / "legacy", root / "ir")
            entries = [
                {
                    "jmhVersion": "1.37",
                    "benchmark": BENCHMARK_PREFIX + case.name,
                    "mode": "avgt",
                    "threads": 1,
                    "forks": 1,
                    "jvm": "/jdk/bin/java",
                    "jvmArgs": [
                        f"-Djava.library.path={prepared_roots[0] / 'generated'}",
                        "--enable-native-access=ALL-UNNAMED",
                        "-Xlog:library=info",
                    ],
                    "jdkVersion": "25",
                    "vmName": "OpenJDK",
                    "vmVersion": "25",
                    "warmupIterations": 3,
                    "warmupTime": "1 s",
                    "warmupBatchSize": 1,
                    "measurementIterations": 3,
                    "measurementTime": "1 s",
                    "measurementBatchSize": 1,
                    "primaryMetric": {
                        "score": 1.0,
                        "scoreUnit": "ns/op",
                        "rawData": [[1.0, 1.0, 1.0]],
                    },
                }
                for case in PRIMITIVE_CASES
            ]
            path = root / "results.json"
            path.write_text(json.dumps(entries))

            loaded, _, _ = validate_result(
                ComparisonRun(1, "a", "legacy"),
                path,
                prepared_roots,
                SUITES["primitive"],
            )
            entries[0]["measurementIterations"] = 2
            path.write_text(json.dumps(entries))

            self.assertEqual(len(PRIMITIVE_CASES), len(loaded))
            with self.assertRaises(SystemExit):
                validate_result(
                    ComparisonRun(1, "a", "legacy"),
                    path,
                    prepared_roots,
                    SUITES["primitive"],
                )

    def test_rejects_fallback_and_missing_fork_library_loads(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            generated = root / "generated"
            generated.mkdir()
            library = generated / "libdemo_jni.dylib"
            library.write_bytes(b"jni")
            prepared = {
                "native_library": artifact_record(
                    library, "generated/libdemo_jni.dylib"
                )
            }
            expected = f"[0.1s][info][library] Loaded library {library}, handle 0x1"
            fallback = f"[0.1s][info][library] Loaded library {generated / 'libdemo.dylib'}, handle 0x2"

            def assert_invalid(last_fork: str) -> None:
                log = root / "run.log"
                log.write_text(
                    "\n".join(
                        f"# Fork: 1 of 1\n{line}"
                        for line in [expected] * 20 + [last_fork]
                    )
                )
                with self.assertRaises(SystemExit):
                    validate_loaded_library(
                        ComparisonRun(1, "a", "ir"),
                        log,
                        root,
                        prepared,
                        SUITES["primitive"],
                    )

            assert_invalid(fallback)
            assert_invalid("")

    def test_verdict_is_enforced_after_provenance_is_written(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            provenance = Path(temporary_directory) / "comparison.json"
            bounds = {case.name: 1.0 for case in PRIMITIVE_CASES}
            provenance.write_text(
                json.dumps(
                    {
                        "design": {"suite": "primitive"},
                        "non_inferiority": {
                            "margin": 1.05,
                            "established": True,
                            "inconclusive_cases": [],
                            "upper_ratio_bounds": bounds,
                        }
                    }
                )
            )

            enforce_verdict(argparse.Namespace(provenance=provenance))
            bounds["echo_bool"] = 1.06
            provenance.write_text(
                json.dumps(
                    {
                        "design": {"suite": "primitive"},
                        "non_inferiority": {
                            "margin": 1.05,
                            "established": False,
                            "inconclusive_cases": ["echo_bool"],
                            "upper_ratio_bounds": bounds,
                        }
                    }
                )
            )
            with self.assertRaises(SystemExit):
                enforce_verdict(argparse.Namespace(provenance=provenance))

    def test_verdict_rejects_missing_and_inconsistent_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            provenance = Path(temporary_directory) / "comparison.json"
            provenance.write_text("{}")
            with self.assertRaises(SystemExit):
                enforce_verdict(argparse.Namespace(provenance=provenance))
            provenance.write_text(
                json.dumps(
                    {
                        "design": {"suite": "primitive"},
                        "non_inferiority": {
                            "margin": 1.05,
                            "established": False,
                            "inconclusive_cases": [],
                            "upper_ratio_bounds": {
                                case.name: 1.0 for case in PRIMITIVE_CASES
                            },
                        }
                    }
                )
            )
            with self.assertRaises(SystemExit):
                enforce_verdict(argparse.Namespace(provenance=provenance))

    def test_verdict_rejects_changed_margin(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            provenance = Path(temporary_directory) / "comparison.json"
            provenance.write_text(
                json.dumps(
                    {
                        "design": {"suite": "primitive"},
                        "non_inferiority": {
                            "margin": 100,
                            "established": True,
                            "inconclusive_cases": [],
                            "upper_ratio_bounds": {
                                case.name: 1.0 for case in PRIMITIVE_CASES
                            },
                        }
                    }
                )
            )
            with self.assertRaises(SystemExit):
                enforce_verdict(argparse.Namespace(provenance=provenance))


if __name__ == "__main__":
    unittest.main()
