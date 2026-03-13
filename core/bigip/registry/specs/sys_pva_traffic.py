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
            "sys_pva_traffic",
            module="sys",
            object_types=("pva-traffic",),
        ),
        header_types=(("sys", "pva-traffic"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
