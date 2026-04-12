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
            "sys_alert_lcd",
            module="sys",
            object_types=("alert lcd",),
        ),
        header_types=(("sys", "alert lcd"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
