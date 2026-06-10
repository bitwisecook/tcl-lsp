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
            "cli_transaction",
            module="cli",
            object_types=("transaction",),
        ),
        header_types=(("cli", "transaction"),),
        properties=(BigipPropertySpec(name="submit", value_type="unknown"),),
    )
