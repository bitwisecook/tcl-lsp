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
            "security_http_mandatory_header",
            module="security",
            object_types=("http mandatory-header",),
        ),
        header_types=(("security", "http mandatory-header"),),
    )
