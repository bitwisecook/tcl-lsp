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
            "ltm_profile_wa_cache",
            module="ltm",
            object_types=("profile wa-cache",),
        ),
        header_types=(("ltm", "profile wa-cache"),),
    )
