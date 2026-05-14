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
            "analytics_gtm_wideip_report",
            module="analytics",
            object_types=("gtm-wideip report",),
        ),
        header_types=(("analytics", "gtm-wideip report"),),
        properties=(),
    )
