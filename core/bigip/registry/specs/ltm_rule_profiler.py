from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "ltm_rule_profiler",
            module="ltm",
            object_types=("rule-profiler",),
        ),
        header_types=(("ltm", "rule-profiler"),),
        properties=(),
    )
