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
            "analytics_protocol_security_http_report",
            module="analytics",
            object_types=("protocol-security-http report",),
        ),
        header_types=(("analytics", "protocol-security-http report"),),
        properties=(),
    )
