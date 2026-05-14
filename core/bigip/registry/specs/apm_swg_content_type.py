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
            "apm_swg_content_type",
            module="apm",
            object_types=("swg-content-type",),
        ),
        header_types=(("apm", "swg-content-type"),),
        properties=(),
    )
