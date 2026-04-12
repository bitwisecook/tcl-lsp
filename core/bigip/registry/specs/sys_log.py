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
            "sys_log",
            module="sys",
            object_types=("log",),
        ),
        header_types=(("sys", "log"),),
        properties=(
            BigipPropertySpec(name="security", value_type="reference", references=("auth_user",)),
            BigipPropertySpec(name="lines", value_type="integer"),
            BigipPropertySpec(name="range", value_type="string"),
        ),
    )
