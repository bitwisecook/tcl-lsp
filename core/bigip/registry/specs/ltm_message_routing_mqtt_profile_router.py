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
                references=("ltm_message_routing_mqtt_profile_router",),
                default="router",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="max-payload-pending-bytes",
                value_type="integer",
                default="32768",
            ),
            BigipPropertySpec(name="max-pending-bytes", value_type="integer", default="32768"),
            BigipPropertySpec(name="max-pending-messages", value_type="integer", default="64"),
            BigipPropertySpec(name="max-retries", value_type="integer", default="1"),
            BigipPropertySpec(
                name="per-peer-stats",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="route",
                value_type="list",
                allow_none=True,
                references=(
                    "apm_policy_agent_route_domain_selection",
                    "ltm_message_routing_diameter_route",
                    "ltm_message_routing_generic_route",
                    "ltm_message_routing_mqtt_route",
                    "ltm_message_routing_sip_route",
                    "net_route",
                    "net_route_domain",
                    "net_routing_route_map",
                    "security_flowspec_route_injector_flowspec_advertised_route_info",
                    "security_flowspec_route_injector_profile",
                    "sys_management_route",
                    "wom_advertised_route",
                    "wom_remote_route",
                ),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="traffic-group",
                value_type="string",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
            BigipPropertySpec(
                name="use-local-connection",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
        ),
    )
