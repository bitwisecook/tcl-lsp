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
            "sys_pfman_consumer",
            module="sys",
            object_types=("pfman consumer",),
        ),
        header_types=(("sys", "pfman consumer"),),
        properties=(
            BigipPropertySpec(name="state", value_type="enum", enum_values=("up", "down", "reset")),
        ),
    )
