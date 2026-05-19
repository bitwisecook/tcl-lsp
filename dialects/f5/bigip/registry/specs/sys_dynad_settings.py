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
            "sys_dynad_settings",
            module="sys",
            object_types=("dynad settings",),
        ),
        header_types=(("sys", "dynad settings"),),
        properties=(BigipPropertySpec(name="development-mode", value_type="unknown"),),
    )
