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
            "sys_software_image",
            module="sys",
            object_types=("software image",),
        ),
        header_types=(("sys", "software image"),),
        properties=(
            BigipPropertySpec(name="image", value_type="string"),
            BigipPropertySpec(name="volume", value_type="string"),
        ),
    )
