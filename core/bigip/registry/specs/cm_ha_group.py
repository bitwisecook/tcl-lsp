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
            "cm_ha_group",
            module="cm",
            object_types=("ha-group",),
        ),
        header_types=(("cm", "ha-group"),),
    )
