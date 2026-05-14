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
            "security_ip_intelligence_info",
            module="security",
            object_types=("ip-intelligence info",),
        ),
        header_types=(("security", "ip-intelligence info"),),
        properties=(),
    )
