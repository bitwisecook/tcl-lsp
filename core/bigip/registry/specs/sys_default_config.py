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
            "sys_default_config",
            module="sys",
            object_types=("default-config",),
        ),
        header_types=(("sys", "default-config"),),
        properties=(BigipPropertySpec(name="load", value_type="string"),),
    )
