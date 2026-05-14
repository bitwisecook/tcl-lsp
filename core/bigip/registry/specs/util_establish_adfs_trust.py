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
            "util_establish_adfs_trust",
            module="util",
            object_types=("establish adfs trust",),
        ),
        header_types=(("util", "establish adfs trust"),),
        properties=(),
    )
