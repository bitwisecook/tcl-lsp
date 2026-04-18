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
            "sys_icontrol_soap",
            module="sys",
            object_types=("icontrol-soap",),
        ),
        header_types=(("sys", "icontrol-soap"),),
        properties=(
            BigipPropertySpec(
                name="allow",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "none", "replace-all-with"),
            ),
        ),
    )
