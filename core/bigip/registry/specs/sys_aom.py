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
            "sys_aom",
            module="sys",
            object_types=("aom",),
        ),
        header_types=(("sys", "aom"),),
        properties=(BigipPropertySpec(name="aom", value_type="string"),),
    )
