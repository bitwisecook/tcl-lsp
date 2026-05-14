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
            "wom_advertised_route",
            module="wom",
            object_types=("advertised-route",),
        ),
        header_types=(("wom", "advertised-route"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dest", value_type="string"),
            BigipPropertySpec(
                name="include",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="label", value_type="unknown"),
            BigipPropertySpec(name="metric", value_type="integer"),
            BigipPropertySpec(
                name="origin",
                value_type="enum",
                enum_values=("configured", "discovered", "manually-saved", "persistable"),
            ),
        ),
    )
