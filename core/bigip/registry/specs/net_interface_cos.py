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
            "net_interface_cos",
            module="net",
            object_types=("interface-cos",),
        ),
        header_types=(("net", "interface-cos"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
