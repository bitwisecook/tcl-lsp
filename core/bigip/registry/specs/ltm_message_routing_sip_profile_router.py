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
            "ltm_message_routing_sip_profile_router",
            module="ltm",
            object_types=("message-routing sip profile router",),
        ),
        header_types=(("ltm", "message-routing sip profile router"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="concurrent-sessions-per-subscriber", value_type="integer"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_message_routing_sip_profile_router",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dialog-establishment-timeout", value_type="integer"),
            BigipPropertySpec(
                name="inherited-traffic-group",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="log-profile",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="log-publisher",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(name="max-global-registrations", value_type="integer"),
            BigipPropertySpec(name="max-pending-bytes", value_type="integer"),
            BigipPropertySpec(name="max-pending-messages", value_type="integer"),
            BigipPropertySpec(name="max-retries", value_type="integer"),
            BigipPropertySpec(name="media-proxy", value_type="unknown"),
            BigipPropertySpec(
                name="max-media-sessions",
                value_type="integer",
                in_sections=("media-proxy",),
            ),
            BigipPropertySpec(
                name="media-inactivity-timeout",
                value_type="integer",
                in_sections=("media-proxy",),
            ),
            BigipPropertySpec(
                name="mirror",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="nonregistered-subscriber-callout",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="nonregistered-subscriber-listener",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="operation-mode",
                value_type="enum",
                enum_values=("application-level-gateway", "load-balancing"),
            ),
            BigipPropertySpec(
                name="per-peer-stats",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="registration-timeout", value_type="integer"),
            BigipPropertySpec(
                name="routes",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="session", value_type="unknown"),
            BigipPropertySpec(
                name="max-session-timeout",
                value_type="integer",
                in_sections=("session",),
            ),
            BigipPropertySpec(
                name="transaction-timeout",
                value_type="integer",
                in_sections=("session",),
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
            ),
        ),
    )
