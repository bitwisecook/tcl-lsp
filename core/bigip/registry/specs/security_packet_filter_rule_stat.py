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
            "security_packet_filter_rule_stat",
            module="security",
            object_types=("packet-filter rule-stat",),
        ),
        header_types=(("security", "packet-filter rule-stat"),),
    )
