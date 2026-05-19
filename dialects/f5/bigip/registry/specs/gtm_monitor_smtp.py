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
            "gtm_monitor_smtp",
            module="gtm",
            object_types=("monitor smtp",),
        ),
        header_types=(("gtm", "monitor smtp"),),
        properties=(
            BigipPropertySpec(
                name="debug",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("gtm_monitor_smtp",),
                default="smtp",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination",
                value_type="string",
                shape_kind="endpoint",
                default="*:*",
            ),
            BigipPropertySpec(
                name="domain",
                value_type="reference",
                allow_none=True,
                references=(
                    "apm_policy_agent_route_domain_selection",
                    "cm_trust_domain",
                    "net_route_domain",
                    "security_firewall_user_domain",
                    "wam_domain_list",
                    "wam_resource_domain_list",
                ),
                default="none",
            ),
            BigipPropertySpec(
                name="ignore-down-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="interval", value_type="integer", default="30 seconds"),
            BigipPropertySpec(name="probe-timeout", value_type="integer", default="5 seconds"),
            BigipPropertySpec(name="timeout", value_type="integer", default="120 seconds"),
        ),
    )
