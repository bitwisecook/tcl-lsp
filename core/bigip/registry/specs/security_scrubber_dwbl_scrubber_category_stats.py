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
            "security_scrubber_dwbl_scrubber_category_stats",
            module="security",
            object_types=("scrubber dwbl-scrubber-category-stats",),
        ),
        header_types=(("security", "scrubber dwbl-scrubber-category-stats"),),
    )
