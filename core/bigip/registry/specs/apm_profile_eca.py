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
            "apm_profile_eca",
            module="apm",
            object_types=("profile eca",),
        ),
        header_types=(("apm", "profile eca"),),
        properties=(),
    )
