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
            "apm_license",
            module="apm",
            object_types=("license",),
        ),
        header_types=(("apm", "license"),),
        properties=(),
    )
