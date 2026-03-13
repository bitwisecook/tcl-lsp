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
            BigipPropertySpec(name="defaults-from", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
