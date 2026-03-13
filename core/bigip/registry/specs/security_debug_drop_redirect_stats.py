from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "security_debug_drop_redirect_stats",
            module="security",
            object_types=("debug drop-redirect-stats",),
        ),
        header_types=(("security", "debug drop-redirect-stats"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
