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
            "sys_ip_stat",
            module="sys",
            object_types=("ip-stat",),
        ),
        header_types=(("sys", "ip-stat"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
