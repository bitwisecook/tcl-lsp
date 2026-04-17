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
            "ltm_message_routing_diameter_profile_router",
            module="ltm",
            object_types=("message-routing diameter profile router",),
        ),
        header_types=(("ltm", "message-routing diameter profile router"),),
        properties=(
            BigipPropertySpec(
                name="associate-clientside-to-poolmember",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="defaults-from", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="ha-message-sweeper-interval", value_type="integer"),
            BigipPropertySpec(
                name="ignore-peer-port",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="irule-scope-message", value_type="enum", enum_values=("yes", "no")
            ),
            BigipPropertySpec(name="max-pending-bytes", value_type="integer"),
            BigipPropertySpec(name="max-pending-messages", value_type="integer"),
            BigipPropertySpec(name="max-retries", value_type="integer"),
            BigipPropertySpec(
                name="mirrored", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="pending-request-sweeper-interval", value_type="integer"),
            BigipPropertySpec(
                name="per-peer-stats", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="routes",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "default", "delete", "none", "replace-all-with"),
            ),
            BigipPropertySpec(name="supported-applications", value_type="integer", repeated=True),
            BigipPropertySpec(
                name="traffic-group",
                value_type="reference",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
            BigipPropertySpec(name="transaction-timeout", value_type="integer"),
            BigipPropertySpec(
                name="use-local-connection", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
