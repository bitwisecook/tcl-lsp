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
            "ltm_snat_translation",
            module="ltm",
            object_types=("snat-translation",),
        ),
        header_types=(("ltm", "snat-translation"),),
        properties=(
            BigipPropertySpec(
                name="address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="arp", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="connection-limit", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="ip-idle-timeout", value_type="integer"),
            BigipPropertySpec(name="tcp-idle-timeout", value_type="integer"),
            BigipPropertySpec(name="udp-idle-timeout", value_type="integer"),
            BigipPropertySpec(
                name="traffic-group",
                value_type="reference",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
        ),
    )
