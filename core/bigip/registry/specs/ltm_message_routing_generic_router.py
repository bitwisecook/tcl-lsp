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
                references=("ltm_message_routing_generic_router",),
                default="messagerouter",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="ha-message-sweeper-interval",
                value_type="integer",
                default="1000 milliseconds",
            ),
            BigipPropertySpec(
                name="ignore-client-port",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="irule-scope-message",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(name="max-pending-bytes", value_type="integer", default="32768"),
            BigipPropertySpec(name="max-pending-messages", value_type="integer", default="64"),
            BigipPropertySpec(name="max-retries", value_type="integer", default="1"),
            BigipPropertySpec(
                name="mirrored",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="per-peer-stats",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
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
                shape_kind="boolean",
            ),
        ),
    )
