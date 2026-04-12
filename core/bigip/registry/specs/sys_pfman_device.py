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
            "sys_pfman_device",
            module="sys",
            object_types=("pfman device",),
        ),
        header_types=(("sys", "pfman device"),),
        properties=(
            BigipPropertySpec(name="state", value_type="enum", enum_values=("up", "down", "reset")),
        ),
    )
