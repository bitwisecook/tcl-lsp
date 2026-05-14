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
            "analytics_sip_report",
            module="analytics",
            object_types=("sip report",),
        ),
        header_types=(("analytics", "sip report"),),
        properties=(),
    )
