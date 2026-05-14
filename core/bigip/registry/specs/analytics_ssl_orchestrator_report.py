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
            "analytics_ssl_orchestrator_report",
            module="analytics",
            object_types=("ssl-orchestrator report",),
        ),
        header_types=(("analytics", "ssl-orchestrator report"),),
        properties=(),
    )
