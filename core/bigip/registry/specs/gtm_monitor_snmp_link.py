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
            "gtm_monitor_snmp_link",
            module="gtm",
            object_types=("monitor snmp-link",),
        ),
        header_types=(("gtm", "monitor snmp-link"),),
        properties=(
            BigipPropertySpec(name="community", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("gtm_monitor_snmp_link",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(
                name="ignore-down-response", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(
                name="port", value_type="integer", allow_none=True, min_value=0, max_value=65535
            ),
            BigipPropertySpec(name="probe", value_type="integer"),
            BigipPropertySpec(name="probe-interval", value_type="integer"),
            BigipPropertySpec(name="probe-timeout", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="version", value_type="integer", allow_none=True),
        ),
    )
