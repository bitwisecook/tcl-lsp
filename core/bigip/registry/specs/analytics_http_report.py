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
            "analytics_http_report",
            module="analytics",
            object_types=("http report",),
        ),
        header_types=(("analytics", "http report"),),
        properties=(),
    )
