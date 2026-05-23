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
            "sys_iapprestricted_key",
            module="sys",
            object_types=("iapprestricted key",),
        ),
        header_types=(("sys", "iapprestricted key"),),
        properties=(BigipPropertySpec(name="restricted-key", value_type="string"),),
    )
