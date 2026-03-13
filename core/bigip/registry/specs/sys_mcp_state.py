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
            "sys_mcp_state",
            module="sys",
            object_types=("mcp-state",),
        ),
        header_types=(("sys", "mcp-state"),),
    )
