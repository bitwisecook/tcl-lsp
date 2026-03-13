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
            "sys_dynad_key",
            module="sys",
            object_types=("dynad key",),
        ),
        header_types=(("sys", "dynad key"),),
        properties=(BigipPropertySpec(name="generate", value_type="string"),),
    )
