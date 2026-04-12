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
            "sys_raid_bay",
            module="sys",
            object_types=("raid bay",),
        ),
        header_types=(("sys", "raid bay"),),
    )
