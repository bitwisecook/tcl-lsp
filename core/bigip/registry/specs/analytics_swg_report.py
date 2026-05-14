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
            "analytics_swg_report",
            module="analytics",
            object_types=("swg report",),
        ),
        header_types=(("analytics", "swg report"),),
        properties=(),
    )
