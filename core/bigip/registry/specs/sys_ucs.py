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
            "sys_ucs",
            module="sys",
            object_types=("ucs",),
        ),
        header_types=(("sys", "ucs"),),
        properties=(
            BigipPropertySpec(name="save", value_type="string"),
            BigipPropertySpec(name="load", value_type="string"),
        ),
    )
