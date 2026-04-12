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
            "net_ipv6_subscriber_prefix_length",
            module="net",
            object_types=("ipv6-subscriber-prefix-length",),
        ),
        header_types=(("net", "ipv6-subscriber-prefix-length"),),
        properties=(BigipPropertySpec(name="value", value_type="string"),),
    )
