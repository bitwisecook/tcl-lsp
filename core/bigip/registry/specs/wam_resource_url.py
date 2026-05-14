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
            "wam_resource_url",
            module="wam",
            object_types=("resource url",),
        ),
        header_types=(("wam", "resource url"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="type", value_type="enum", enum_values=("css", "js")),
            BigipPropertySpec(name="url", value_type="unknown"),
        ),
    )
