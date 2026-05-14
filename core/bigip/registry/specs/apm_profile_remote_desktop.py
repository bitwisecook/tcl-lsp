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
            "apm_profile_remote_desktop",
            module="apm",
            object_types=("profile remote-desktop",),
        ),
        header_types=(("apm", "profile remote-desktop"),),
        properties=(),
    )
