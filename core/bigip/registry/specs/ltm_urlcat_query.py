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
            "ltm_urlcat_query",
            module="ltm",
            object_types=("urlcat-query",),
        ),
        header_types=(("ltm", "urlcat-query"),),
    )
