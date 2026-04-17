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
            "gtm_topology",
            module="gtm",
            object_types=("topology",),
        ),
        header_types=(("gtm", "topology"),),
        properties=(
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="order", value_type="integer"),
            BigipPropertySpec(name="score", value_type="integer"),
            BigipPropertySpec(name="region", value_type="string"),
        ),
    )
