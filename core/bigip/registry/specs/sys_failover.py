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
            "sys_failover",
            module="sys",
            object_types=("failover",),
        ),
        header_types=(("sys", "failover"),),
        properties=(
            BigipPropertySpec(name="device", value_type="string"),
            BigipPropertySpec(
                name="traffic-group",
                value_type="reference",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
        ),
    )
