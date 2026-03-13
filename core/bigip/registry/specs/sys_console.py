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
            "sys_console",
            module="sys",
            object_types=("console",),
        ),
        header_types=(("sys", "console"),),
        properties=(BigipPropertySpec(name="baud-rate", value_type="integer"),),
    )
