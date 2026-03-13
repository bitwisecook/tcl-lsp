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
            "sys_software_volume",
            module="sys",
            object_types=("software volume",),
        ),
        header_types=(("sys", "software volume"),),
        properties=(
            BigipPropertySpec(name="reboot", value_type="string"),
            BigipPropertySpec(name="media", value_type="string"),
        ),
    )
