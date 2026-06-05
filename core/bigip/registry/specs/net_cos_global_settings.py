from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "net_cos_global_settings",
            module="net",
            object_types=("cos global-settings",),
        ),
        header_types=(("net", "cos global-settings"),),
        properties=(
            BigipPropertySpec(name="default-map-8021p", value_type="unknown"),
            BigipPropertySpec(name="default-map-dscp", value_type="unknown"),
            BigipPropertySpec(name="default-traffic-priority", value_type="unknown"),
            BigipPropertySpec(name="feature-disabled", value_type="unknown"),
            BigipPropertySpec(name="feature-enabled", value_type="unknown"),
            BigipPropertySpec(name="precedence", value_type="unknown"),
        ),
    )
