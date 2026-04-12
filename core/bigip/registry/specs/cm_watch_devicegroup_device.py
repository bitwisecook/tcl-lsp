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
            "cm_watch_devicegroup_device",
            module="cm",
            object_types=("watch-devicegroup-device",),
        ),
        header_types=(("cm", "watch-devicegroup-device"),),
    )
