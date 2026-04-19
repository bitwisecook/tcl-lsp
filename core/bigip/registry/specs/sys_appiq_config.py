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
            "sys_appiq_config",
            module="sys",
            object_types=("appiq config",),
        ),
        header_types=(("sys", "appiq config"),),
        properties=(BigipPropertySpec(name="host-ip", value_type="string"),),
    )
