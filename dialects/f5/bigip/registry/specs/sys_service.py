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
            "sys_service",
            module="sys",
            object_types=("service",),
        ),
        header_types=(("sys", "service"),),
        properties=(
            BigipPropertySpec(name="force", value_type="unknown"),
            BigipPropertySpec(name="restart", value_type="reference", references=("restart",)),
            BigipPropertySpec(name="start", value_type="reference", references=("start",)),
            BigipPropertySpec(name="stop", value_type="reference", references=("stop",)),
        ),
    )
