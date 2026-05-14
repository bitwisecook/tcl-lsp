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
            "analytics_dos_vis_attacks_report",
            module="analytics",
            object_types=("dos-vis-attacks report",),
        ),
        header_types=(("analytics", "dos-vis-attacks report"),),
        properties=(),
    )
