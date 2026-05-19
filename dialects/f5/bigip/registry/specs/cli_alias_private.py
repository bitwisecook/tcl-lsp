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
            "cli_alias_private",
            module="cli",
            object_types=("alias private",),
        ),
        header_types=(("cli", "alias private"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="command", value_type="unknown", repeated=True),
            BigipPropertySpec(name="description", value_type="string"),
        ),
    )
