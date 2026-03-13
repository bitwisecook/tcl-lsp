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
            "ltm_tacdb_licenseddb",
            module="ltm",
            object_types=("tacdb licenseddb",),
        ),
        header_types=(("ltm", "tacdb licenseddb"),),
        properties=(
            BigipPropertySpec(name="poll-interval", value_type="integer"),
            BigipPropertySpec(name="load", value_type="string"),
        ),
    )
