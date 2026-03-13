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
            "security_firewall_matching_rule",
            module="security",
            object_types=("firewall matching-rule",),
        ),
        header_types=(("security", "firewall matching-rule"),),
        properties=(
            BigipPropertySpec(name="dest-addr", value_type="string"),
            BigipPropertySpec(name="source-addr", value_type="string"),
            BigipPropertySpec(name="dest-port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(
                name="source-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(name="protocol", value_type="string"),
            BigipPropertySpec(name="vlan", value_type="reference", references=("net_vlan",)),
        ),
    )
