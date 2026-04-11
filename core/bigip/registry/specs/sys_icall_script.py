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
            "sys_icall_script",
            module="sys",
            object_types=("icall script",),
        ),
        header_types=(("sys", "icall script"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="events",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="contexts",
                value_type="enum",
                in_sections=("events",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
        ),
    )
