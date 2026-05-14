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
            "wam_roi_statistics",
            module="wam",
            object_types=("roi-statistics",),
        ),
        header_types=(("wam", "roi-statistics"),),
        properties=(),
    )
