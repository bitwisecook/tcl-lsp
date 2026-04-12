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
            BigipPropertySpec(name="concurrent-sessions-per-subscriber", value_type="integer"),
            BigipPropertySpec(name="defaults-from", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dialog-establishment-timeout", value_type="integer"),
            BigipPropertySpec(
                name="inherited-traffic-group", value_type="enum", enum_values=("true", "false")
            ),
            BigipPropertySpec(name="log-profile", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="log-publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="max-global-registrations", value_type="integer"),
            BigipPropertySpec(name="max-pending-bytes", value_type="integer"),
            BigipPropertySpec(name="max-pending-messages", value_type="integer"),
            BigipPropertySpec(name="max-retries", value_type="integer"),
            BigipPropertySpec(name="media-proxy", value_type="string"),
            BigipPropertySpec(
                name="max-media-sessions", value_type="integer", in_sections=("media-proxy",)
            ),
            BigipPropertySpec(
                name="media-inactivity-timeout", value_type="integer", in_sections=("media-proxy",)
            ),
            BigipPropertySpec(
                name="mirror", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="nonregistered-subscriber-callout",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="nonregistered-subscriber-listener",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="per-peer-stats", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="registration-timeout", value_type="integer"),
            BigipPropertySpec(
                name="operation-mode",
                value_type="enum",
                enum_values=("load-balancing", "application-level-gateway"),
            ),
            BigipPropertySpec(
                name="routes",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "default", "delete", "none", "replace-all-with"),
            ),
            BigipPropertySpec(name="session", value_type="string"),
            BigipPropertySpec(
                name="transaction-timeout", value_type="integer", in_sections=("session",)
            ),
            BigipPropertySpec(
                name="max-session-timeout", value_type="integer", in_sections=("session",)
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
