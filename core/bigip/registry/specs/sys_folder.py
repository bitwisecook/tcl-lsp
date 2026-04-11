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
            "sys_folder",
            module="sys",
            object_types=("folder",),
        ),
        header_types=(("sys", "folder"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="device-group",
                value_type="reference",
                allow_none=True,
                references=("cm_device_group",),
            ),
            BigipPropertySpec(
                name="no-ref-check", value_type="enum", enum_values=("false", "true")
            ),
            BigipPropertySpec(
                name="traffic-group",
                value_type="reference",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
        ),
    )
