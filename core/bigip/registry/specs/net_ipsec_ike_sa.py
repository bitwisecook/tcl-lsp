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
            "net_ipsec_ike_sa",
            module="net",
            object_types=("ipsec ike-sa",),
        ),
        header_types=(("net", "ipsec ike-sa"),),
        properties=(),
    )
