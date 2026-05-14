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
            "analytics_asm_learning_suggestions_report",
            module="analytics",
            object_types=("asm-learning-suggestions report",),
        ),
        header_types=(("analytics", "asm-learning-suggestions report"),),
        properties=(),
    )
