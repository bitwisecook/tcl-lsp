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
            "ltm_message_routing_diameter_route",
            module="ltm",
            object_types=("message-routing diameter route",),
        ),
        header_types=(("ltm", "message-routing diameter route"),),
        properties=(
            BigipPropertySpec(name="application-id", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination-realm", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="origin-realm", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="peer-selection-mode", value_type="enum", enum_values=("ratio", "sequential")
            ),
            BigipPropertySpec(name="peers", value_type="boolean", repeated=True, allow_none=True),
            BigipPropertySpec(name="virtual-server", value_type="string"),
        ),
    )
