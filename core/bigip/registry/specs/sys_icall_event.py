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
            "sys_icall_event",
            module="sys",
            object_types=("icall event",),
        ),
        header_types=(("sys", "icall event"),),
        properties=(),
    )
