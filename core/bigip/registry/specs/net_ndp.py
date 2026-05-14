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
            "net_ndp",
            module="net",
            object_types=("ndp",),
        ),
        header_types=(("net", "ndp"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="ip-address", value_type="string"),
            BigipPropertySpec(name="mac-address", value_type="unknown"),
        ),
    )
