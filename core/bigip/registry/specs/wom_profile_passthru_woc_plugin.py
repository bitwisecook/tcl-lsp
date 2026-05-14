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
            "wom_profile_passthru_woc_plugin",
            module="wom",
            object_types=("profile passthru-woc-plugin",),
        ),
        header_types=(("wom", "profile passthru-woc-plugin"),),
        properties=(),
    )
