from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "sys_config_diff",
            module="sys",
            object_types=("config-diff",),
        ),
        header_types=(("sys", "config-diff"),),
    )
