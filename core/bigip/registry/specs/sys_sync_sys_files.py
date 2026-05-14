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
            "sys_sync_sys_files",
            module="sys",
            object_types=("sync-sys-files",),
        ),
        header_types=(("sys", "sync-sys-files"),),
        properties=(),
    )
