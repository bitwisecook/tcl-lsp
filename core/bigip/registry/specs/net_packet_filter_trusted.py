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
            "net_packet_filter_trusted",
            module="net",
            object_types=("packet-filter-trusted",),
        ),
        header_types=(("net", "packet-filter-trusted"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="ip-addresses", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="mac-addresses", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="vlans", value_type="unknown", allow_none=True),
        ),
    )
