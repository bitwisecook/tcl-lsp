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
            "ltm_message_routing_mqtt_peer",
            module="ltm",
            object_types=("message-routing mqtt peer",),
        ),
        header_types=(("ltm", "message-routing mqtt peer"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="pool", value_type="reference", references=("ltm_pool",)),
            BigipPropertySpec(name="transport-config", value_type="string"),
        ),
    )
