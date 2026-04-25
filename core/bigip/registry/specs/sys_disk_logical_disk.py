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
            "sys_disk_logical_disk",
            module="sys",
            object_types=("disk logical-disk",),
        ),
        header_types=(("sys", "disk logical-disk"),),
        properties=(
            BigipPropertySpec(name="vg-reserved", value_type="integer"),
            BigipPropertySpec(name="mode", value_type="boolean", allow_none=True),
        ),
    )
