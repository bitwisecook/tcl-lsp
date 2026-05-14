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
            "wam_resource_concat_set",
            module="wam",
            object_types=("resource concat-set",),
        ),
        header_types=(("wam", "resource concat-set"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="members", value_type="unknown"),
            BigipPropertySpec(name="type", value_type="enum", enum_values=("css", "js")),
            BigipPropertySpec(name="url", value_type="unknown"),
        ),
    )
