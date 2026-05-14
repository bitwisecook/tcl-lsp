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
            "apm_profile_oauthplugin",
            module="apm",
            object_types=("profile oauthplugin",),
        ),
        header_types=(("apm", "profile oauthplugin"),),
        properties=(),
    )
