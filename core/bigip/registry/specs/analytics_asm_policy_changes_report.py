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
            "analytics_asm_policy_changes_report",
            module="analytics",
            object_types=("asm-policy-changes report",),
        ),
        header_types=(("analytics", "asm-policy-changes report"),),
        properties=(),
    )
