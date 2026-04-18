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
            "sys_nethsm_sync_queue_stat",
            module="sys",
            object_types=("nethsm sync-queue-stat",),
        ),
        header_types=(("sys", "nethsm sync-queue-stat"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
