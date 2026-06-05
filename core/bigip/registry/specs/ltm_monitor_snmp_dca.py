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
                default="ucd",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="community",
                value_type="reference",
                allow_none=True,
                references=("net_routing_community_list",),
                default="public",
            ),
            BigipPropertySpec(
                name="cpu-coefficient",
                value_type="integer",
                allow_none=True,
                default="1",
            ),
            BigipPropertySpec(name="cpu-threshold", value_type="integer", default="80 percent"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_snmp_dca",),
                default="snmp_dca",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="disk-coefficient",
                value_type="integer",
                allow_none=True,
                default="2",
            ),
            BigipPropertySpec(name="disk-threshold", value_type="integer", default="90 percent"),
            BigipPropertySpec(name="interval", value_type="integer", default="10 seconds"),
            BigipPropertySpec(
                name="memory-coefficient",
                value_type="integer",
                allow_none=True,
                default="1",
            ),
            BigipPropertySpec(name="memory-threshold", value_type="integer", default="70 percent"),
            BigipPropertySpec(name="time-until-up", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="timeout", value_type="integer", default="30 seconds"),
            BigipPropertySpec(name="user-defined", value_type="unknown", default="none"),
            BigipPropertySpec(
                name="version",
                value_type="integer",
                allow_none=True,
                default="none",
            ),
        ),
    )
