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
            "ltm_message_routing_mqtt_route",
            module="ltm",
            object_types=("message-routing mqtt route",),
        ),
        header_types=(("ltm", "message-routing mqtt route"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="peer", value_type="string"),
            BigipPropertySpec(name="virtual-server", value_type="string"),
        ),
    )
