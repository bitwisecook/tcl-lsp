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
            "sys_disk_directory",
            module="sys",
            object_types=("disk directory",),
        ),
        header_types=(("sys", "disk directory"),),
        properties=(BigipPropertySpec(name="new-size", value_type="string"),),
    )
