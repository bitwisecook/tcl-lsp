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
            "ltm_nat_stats",
            module="ltm",
            object_types=("nat-stats",),
        ),
        header_types=(("ltm", "nat-stats"),),
        properties=(),
    )
