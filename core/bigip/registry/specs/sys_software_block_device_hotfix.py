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
            "sys_software_block_device_hotfix",
            module="sys",
            object_types=("software block-device-hotfix",),
        ),
        header_types=(("sys", "software block-device-hotfix"),),
        properties=(),
    )
