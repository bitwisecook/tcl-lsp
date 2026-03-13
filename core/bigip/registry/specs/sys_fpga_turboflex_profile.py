from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "sys_fpga_turboflex_profile",
            module="sys",
            object_types=("fpga turboflex-profile",),
        ),
        header_types=(("sys", "fpga turboflex-profile"),),
    )
