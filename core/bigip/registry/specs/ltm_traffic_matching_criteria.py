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
            "ltm_traffic_matching_criteria",
            module="ltm",
            object_types=("traffic-matching-criteria",),
        ),
        header_types=(("ltm", "traffic-matching-criteria"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination-address-inline",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="destination-address-list",
                value_type="reference",
                allow_none=True,
                default="any",
            ),
            BigipPropertySpec(name="destination-port-inline", value_type="unknown", default="none"),
            BigipPropertySpec(
                name="destination-port-list",
                value_type="reference",
                allow_none=True,
                references=("net_port_list", "security_firewall_port_list"),
                default="any",
            ),
            BigipPropertySpec(name="source-address-inline", value_type="unknown", default="none"),
            BigipPropertySpec(
                name="source-address-list",
                value_type="reference",
                allow_none=True,
                default="any",
            ),
            BigipPropertySpec(name="source-port-inline", value_type="unknown", default="none"),
            BigipPropertySpec(
                name="protocol",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
        ),
    )
