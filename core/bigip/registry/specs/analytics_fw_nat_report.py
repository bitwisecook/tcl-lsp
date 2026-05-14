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
            "analytics_fw_nat_report",
            module="analytics",
            object_types=("fw-nat report",),
        ),
        header_types=(("analytics", "fw-nat report"),),
        properties=(),
    )
