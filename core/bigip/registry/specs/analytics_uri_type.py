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
            "analytics_uri_type",
            module="analytics",
            object_types=("uri-type",),
        ),
        header_types=(("analytics", "uri-type"),),
        properties=(BigipPropertySpec(name="file-extensions", value_type="unknown"),),
    )
