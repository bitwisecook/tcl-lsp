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
            "analytics_application_security_incidents_report",
            module="analytics",
            object_types=("application-security-incidents report",),
        ),
        header_types=(("analytics", "application-security-incidents report"),),
        properties=(),
    )
