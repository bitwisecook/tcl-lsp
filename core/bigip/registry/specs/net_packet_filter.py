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
            "net_packet_filter",
            module="net",
            object_types=("packet-filter",),
        ),
        header_types=(("net", "packet-filter"),),
        properties=(
            BigipPropertySpec(
                name="action",
                value_type="enum",
                enum_values=("accept", "continue", "discard", "reject"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="logging", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="order", value_type="integer"),
            BigipPropertySpec(name="rate-class", value_type="string"),
            BigipPropertySpec(name="rule", value_type="string"),
            BigipPropertySpec(name="vlan", value_type="reference", references=("net_vlan",)),
            BigipPropertySpec(name="reset-stats", value_type="string"),
            BigipPropertySpec(name="order", value_type="string", in_sections=("create",)),
            BigipPropertySpec(name="action", value_type="string", in_sections=("create",)),
            BigipPropertySpec(
                name="vlan",
                value_type="reference",
                in_sections=("create",),
                references=("net_vlan",),
            ),
            BigipPropertySpec(name="logging", value_type="boolean", in_sections=("create",)),
            BigipPropertySpec(name="rule", value_type="string", in_sections=("create",)),
            BigipPropertySpec(name="src", value_type="string", in_sections=("create",)),
            BigipPropertySpec(name="rate", value_type="string", in_sections=("create",)),
        ),
    )
