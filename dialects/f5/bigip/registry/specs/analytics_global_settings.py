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
            "analytics_global_settings",
            module="analytics",
            object_types=("global-settings",),
        ),
        header_types=(("analytics", "global-settings"),),
        properties=(),
    )
