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
            "cm_sniff_updates",
            module="cm",
            object_types=("sniff-updates",),
        ),
        header_types=(("cm", "sniff-updates"),),
    )
