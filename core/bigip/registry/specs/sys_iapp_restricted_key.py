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
            "sys_iapp_restricted_key",
            module="sys",
            object_types=("iapp-restricted-key",),
        ),
        header_types=(("sys", "iapp-restricted-key"),),
        properties=(BigipPropertySpec(name="restricted-key", value_type="string"),),
    )
