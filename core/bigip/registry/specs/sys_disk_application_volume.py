from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "sys_disk_application_volume",
            module="sys",
            object_types=("disk application-volume",),
        ),
        header_types=(("sys", "disk application-volume"),),
    )
