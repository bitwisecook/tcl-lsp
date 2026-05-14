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
            "ltm_message_routing_generic_route",
            module="ltm",
            object_types=("message-routing generic route",),
        ),
        header_types=(("ltm", "message-routing generic route"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination-address", value_type="string"),
            BigipPropertySpec(
                name="peer-selection-mode",
                value_type="enum",
                enum_values=("ratio", "sequential"),
            ),
            BigipPropertySpec(name="peers", value_type="list"),
            BigipPropertySpec(name="source-address", value_type="string"),
        ),
    )
