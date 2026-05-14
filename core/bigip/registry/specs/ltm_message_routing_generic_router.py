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
            "ltm_message_routing_generic_router",
            module="ltm",
            object_types=("message-routing generic router",),
        ),
        header_types=(("ltm", "message-routing generic router"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_message_routing_generic_router",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="ha-message-sweeper-interval", value_type="integer"),
            BigipPropertySpec(
                name="ignore-client-port",
                value_type="enum",
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(
                name="irule-scope-message",
                value_type="enum",
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(name="max-pending-bytes", value_type="integer"),
            BigipPropertySpec(name="max-pending-messages", value_type="integer"),
            BigipPropertySpec(name="max-retries", value_type="integer"),
            BigipPropertySpec(
                name="mirrored",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="per-peer-stats",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="routes", value_type="list"),
            BigipPropertySpec(
                name="traffic-group",
                value_type="string",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
            BigipPropertySpec(
                name="use-local-connection",
                value_type="enum",
                enum_values=("no", "yes"),
            ),
        ),
    )
