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
            "sys_feature_module",
            module="sys",
            object_types=("feature-module",),
        ),
        header_types=(("sys", "feature-module"),),
        properties=(BigipPropertySpec(name="enabled", value_type="boolean"),),
    )
