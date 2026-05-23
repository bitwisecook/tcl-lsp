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
            "vcmp_virtual_disk",
            module="vcmp",
            object_types=("virtual-disk",),
        ),
        header_types=(("vcmp", "virtual-disk"),),
        properties=(),
    )
