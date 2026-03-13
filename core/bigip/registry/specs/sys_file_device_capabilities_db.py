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
            "sys_file_device_capabilities_db",
            module="sys",
            object_types=("file device-capabilities-db",),
        ),
        header_types=(("sys", "file device-capabilities-db"),),
        properties=(BigipPropertySpec(name="source-path", value_type="string"),),
    )
