from __future__ import annotations

import math
import statistics
from dataclasses import dataclass
from typing import Sequence

from .contract import (
    BenchmarkCase,
    NON_INFERIORITY_MARGIN,
    ONE_SIDED_T_95_DF_5,
    PAIRS,
)


@dataclass(frozen=True)
class Inference:
    log_ratios: dict[str, tuple[float, ...]]
    point_ratios: dict[str, float]
    upper_bounds: dict[str, float]

    @property
    def inconclusive(self) -> tuple[str, ...]:
        return tuple(
            case
            for case in sorted(self.upper_bounds)
            if self.upper_bounds[case] > NON_INFERIORITY_MARGIN
        )


def paired_log_upper(log_ratios: Sequence[float]) -> float:
    if len(log_ratios) != 6:
        raise ValueError("six paired log ratios are required")
    mean = statistics.fmean(log_ratios)
    standard_error = statistics.stdev(log_ratios) / math.sqrt(len(log_ratios))
    return math.exp(mean + ONE_SIDED_T_95_DF_5 * standard_error)


def infer(
    scores: dict[str, dict[str, float]], cases: Sequence[BenchmarkCase]
) -> Inference:
    case_names = tuple(case.name for case in cases)
    log_ratios = {
        case: tuple(
            math.log(scores[pair.ir_run][case] / scores[pair.legacy_run][case])
            for pair in PAIRS
        )
        for case in case_names
    }
    return Inference(
        log_ratios=log_ratios,
        point_ratios={
            case: math.exp(statistics.fmean(values))
            for case, values in log_ratios.items()
        },
        upper_bounds={
            case: paired_log_upper(values) for case, values in log_ratios.items()
        },
    )
