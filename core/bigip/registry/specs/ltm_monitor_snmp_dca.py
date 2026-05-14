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
            "ltm_monitor_snmp_dca",
            module="ltm",
            object_types=("monitor snmp-dca",),
        ),
        header_types=(("ltm", "monitor snmp-dca"),),
        properties=(
            BigipPropertySpec(
                name="agent-type",
                value_type="enum",
                enum_values=("generic", "other", "ucd", "win2000"),
            ),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="community",
                value_type="reference",
                allow_none=True,
                references=("net_routing_community_list",),
            ),
            BigipPropertySpec(name="cpu-coefficient", value_type="integer", allow_none=True),
            BigipPropertySpec(name="cpu-threshold", value_type="integer"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_snmp_dca",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="disk-coefficient", value_type="integer", allow_none=True),
            BigipPropertySpec(name="disk-threshold", value_type="integer"),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="memory-coefficient", value_type="integer", allow_none=True),
            BigipPropertySpec(name="memory-threshold", value_type="integer"),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="user-defined", value_type="unknown"),
            BigipPropertySpec(name="version", value_type="integer", allow_none=True),
        ),
    )
