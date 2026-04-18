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
            BigipPropertySpec(name="precedence", value_type="string"),
            BigipPropertySpec(name="default-traffic-priority", value_type="string"),
        ),
    )
