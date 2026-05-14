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
            "analytics_afm_sweeper_report",
            module="analytics",
            object_types=("afm-sweeper report",),
        ),
        header_types=(("analytics", "afm-sweeper report"),),
        properties=(),
    )
