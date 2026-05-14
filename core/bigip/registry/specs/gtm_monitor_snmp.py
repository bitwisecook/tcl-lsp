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
            "gtm_monitor_snmp",
            module="gtm",
            object_types=("monitor snmp",),
        ),
        header_types=(("gtm", "monitor snmp"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="community",
                value_type="reference",
                allow_none=True,
                references=("net_routing_community_list",),
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("gtm_monitor_snmp",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(
                name="ignore-down-response",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="port", value_type="integer"),
            BigipPropertySpec(name="probe", value_type="integer"),
            BigipPropertySpec(name="probe-interval", value_type="integer"),
            BigipPropertySpec(name="probe-timeout", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="version", value_type="integer", allow_none=True),
        ),
    )
