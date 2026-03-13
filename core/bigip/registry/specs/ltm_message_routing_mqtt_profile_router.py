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
            "ltm_message_routing_mqtt_profile_router",
            module="ltm",
            object_types=("message-routing mqtt profile router",),
        ),
        header_types=(("ltm", "message-routing mqtt profile router"),),
        properties=(
            BigipPropertySpec(name="defaults-from", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="max-pending-bytes", value_type="integer"),
            BigipPropertySpec(name="max-payload-pending-bytes", value_type="integer"),
            BigipPropertySpec(name="max-pending-messages", value_type="integer"),
            BigipPropertySpec(name="max-retries", value_type="integer"),
            BigipPropertySpec(
                name="per-peer-stats", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="route",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "default", "delete", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="traffic-group",
                value_type="reference",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
            BigipPropertySpec(
                name="use-local-connection", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
