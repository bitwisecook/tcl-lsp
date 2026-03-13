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
            "net_ike_evt_stat",
            module="net",
            object_types=("ike-evt-stat",),
        ),
        header_types=(("net", "ike-evt-stat"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
