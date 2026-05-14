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
            "apm_profile_ping_access",
            module="apm",
            object_types=("profile ping-access",),
        ),
        header_types=(("apm", "profile ping-access"),),
        properties=(),
    )
