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
            "sys_dynad_rpm",
            module="sys",
            object_types=("dynad rpm",),
        ),
        header_types=(("sys", "dynad rpm"),),
        properties=(
            BigipPropertySpec(name="install", value_type="string"),
            BigipPropertySpec(name="uninstall", value_type="string"),
        ),
    )
