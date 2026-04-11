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
            "sys_software_hotfix",
            module="sys",
            object_types=("software hotfix",),
        ),
        header_types=(("sys", "software hotfix"),),
        properties=(BigipPropertySpec(name="install", value_type="string"),),
    )
