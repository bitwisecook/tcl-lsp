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
            "cli_version",
            module="cli",
            object_types=("version",),
        ),
        header_types=(("cli", "version"),),
        properties=(BigipPropertySpec(name="active", value_type="unknown"),),
    )
