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
            "cm_traffic_group",
            module="cm",
            object_types=("traffic-group",),
        ),
        header_types=(("cm", "traffic-group"),),
        properties=(
            BigipPropertySpec(
                name="auto-failback-enabled", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="auto-failback-time", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="failover-method", value_type="enum", enum_values=("ha-score", "ha-order")
            ),
            BigipPropertySpec(name="ha-group", value_type="reference", references=("cm_ha_group",)),
            BigipPropertySpec(name="ha-load-factor", value_type="integer"),
            BigipPropertySpec(
                name="ha-order", value_type="reference", repeated=True, references=("cm_device",)
            ),
            BigipPropertySpec(name="mac", value_type="string"),
            BigipPropertySpec(
                name="monitor", value_type="reference", repeated=True, references=("cm_ha_group",)
            ),
            BigipPropertySpec(name="unit-id", value_type="integer"),
        ),
    )
