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
            BigipPropertySpec(name="no-persist", value_type="unknown"),
            BigipPropertySpec(name="offline", value_type="unknown"),
            BigipPropertySpec(name="online", value_type="unknown"),
            BigipPropertySpec(name="persist", value_type="unknown"),
            BigipPropertySpec(name="run", value_type="unknown"),
            BigipPropertySpec(name="standby", value_type="unknown"),
            BigipPropertySpec(
                name="traffic-group",
                value_type="string",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
        ),
    )
