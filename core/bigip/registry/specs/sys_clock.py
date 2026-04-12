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
            "sys_clock",
            module="sys",
            object_types=("clock",),
        ),
        header_types=(("sys", "clock"),),
        properties=(BigipPropertySpec(name="time", value_type="string"),),
    )
