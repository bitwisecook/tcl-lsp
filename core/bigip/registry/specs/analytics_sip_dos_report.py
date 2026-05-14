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
            "analytics_sip_dos_report",
            module="analytics",
            object_types=("sip-dos report",),
        ),
        header_types=(("analytics", "sip-dos report"),),
        properties=(),
    )
