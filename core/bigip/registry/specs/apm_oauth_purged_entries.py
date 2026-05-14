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
            "apm_oauth_purged_entries",
            module="apm",
            object_types=("oauth purged-entries",),
        ),
        header_types=(("apm", "oauth purged-entries"),),
        properties=(),
    )
