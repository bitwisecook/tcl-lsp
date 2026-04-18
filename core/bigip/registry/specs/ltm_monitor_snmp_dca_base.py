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
            "ltm_monitor_snmp_dca_base",
            module="ltm",
            object_types=("monitor snmp-dca-base",),
        ),
        header_types=(("ltm", "monitor snmp-dca-base"),),
        properties=(
            BigipPropertySpec(name="community", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="cpu-coefficient", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_snmp_dca_base",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="user-defined", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="version", value_type="integer", allow_none=True),
        ),
    )
