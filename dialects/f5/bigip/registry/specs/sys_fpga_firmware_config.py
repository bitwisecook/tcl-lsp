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
            "sys_fpga_firmware_config",
            module="sys",
            object_types=("fpga firmware-config",),
        ),
        header_types=(("sys", "fpga firmware-config"),),
        properties=(
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=(
                    "l4-performance-fpga",
                    "l7-intelligent-fpga",
                    "standard-balanced-fpga",
                    "traffic-acceleration-fpga",
                ),
            ),
        ),
    )
