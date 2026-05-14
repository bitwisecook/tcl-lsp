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
            "sys_license",
            module="sys",
            object_types=("license",),
        ),
        header_types=(("sys", "license"),),
        properties=(),
    )
