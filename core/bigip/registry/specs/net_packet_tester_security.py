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
            "net_packet_tester_security",
            module="net",
            object_types=("packet-tester security",),
        ),
        header_types=(("net", "packet-tester security"),),
        properties=(),
    )
