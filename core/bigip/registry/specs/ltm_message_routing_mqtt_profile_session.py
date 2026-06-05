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
            "ltm_message_routing_mqtt_profile_session",
            module="ltm",
            object_types=("message-routing mqtt profile session",),
        ),
        header_types=(("ltm", "message-routing mqtt profile session"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_message_routing_mqtt_profile_session",),
                default="session",
            ),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
