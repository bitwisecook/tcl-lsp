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
            "sys_icmp_stat",
            module="sys",
            object_types=("icmp-stat",),
        ),
        header_types=(("sys", "icmp-stat"),),
        properties=(),
    )
