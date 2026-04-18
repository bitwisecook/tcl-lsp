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
            "sys_geoip",
            module="sys",
            object_types=("geoip",),
        ),
        header_types=(("sys", "geoip"),),
        properties=(BigipPropertySpec(name="load", value_type="string"),),
    )
