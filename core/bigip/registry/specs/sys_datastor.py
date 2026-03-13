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
            "sys_datastor",
            module="sys",
            object_types=("datastor",),
        ),
        header_types=(("sys", "datastor"),),
        properties=(
            BigipPropertySpec(name="dedup-cache-weight", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="disk", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="high-water-mark", value_type="integer"),
            BigipPropertySpec(name="low-water-mark", value_type="integer"),
            BigipPropertySpec(name="web-cache-weight", value_type="integer"),
        ),
    )
