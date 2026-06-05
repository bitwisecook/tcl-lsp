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
            "sys_raid_bay",
            module="sys",
            object_types=("raid bay",),
        ),
        header_types=(("sys", "raid bay"),),
        properties=(
            BigipPropertySpec(name="flash-led", value_type="unknown"),
            BigipPropertySpec(name="no-flash-led", value_type="unknown"),
        ),
    )
