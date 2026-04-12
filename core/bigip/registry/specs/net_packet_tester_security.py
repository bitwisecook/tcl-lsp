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
            "net_packet_tester_security",
            module="net",
            object_types=("packet-tester security",),
        ),
        header_types=(("net", "packet-tester security"),),
        properties=(
            BigipPropertySpec(name="dest-addr", value_type="string"),
            BigipPropertySpec(name="source-addr", value_type="string"),
            BigipPropertySpec(name="dest-port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(
                name="source-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(name="protocol", value_type="string"),
            BigipPropertySpec(name="src-vlan", value_type="reference", references=("net_vlan",)),
        ),
    )
