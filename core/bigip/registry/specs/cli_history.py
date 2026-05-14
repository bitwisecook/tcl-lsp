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
            "cli_history",
            module="cli",
            object_types=("history",),
        ),
        header_types=(("cli", "history"),),
        properties=(),
    )
