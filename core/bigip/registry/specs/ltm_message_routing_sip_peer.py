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
            "ltm_message_routing_sip_peer",
            module="ltm",
            object_types=("message-routing sip peer",),
        ),
        header_types=(("ltm", "message-routing sip peer"),),
        properties=(
            BigipPropertySpec(
                name="auto-initialization", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="auto-initialization-interval", value_type="integer"),
            BigipPropertySpec(name="connection-mode", value_type="string"),
            BigipPropertySpec(name="per-client-per-tmm", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="number-connections", value_type="integer"),
            BigipPropertySpec(name="pool", value_type="reference", references=("ltm_pool",)),
            BigipPropertySpec(name="ratio", value_type="integer"),
            BigipPropertySpec(name="transport-config", value_type="string"),
        ),
    )
