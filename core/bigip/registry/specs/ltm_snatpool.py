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
            "ltm_snatpool",
            module="ltm",
            object_types=("snatpool",),
        ),
        header_types=(("ltm", "snatpool"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="members",
                value_type="reference",
                allow_none=True,
                # ``snatpool members`` accepts the same operator family
                # as ``ltm pool members`` (sans ``modify``: snatpool
                # members are bare addresses, no body).
                list_operators=frozenset(("add", "delete", "replace-all-with", "none")),
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
