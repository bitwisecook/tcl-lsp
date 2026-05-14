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
            "net_arp",
            module="net",
            object_types=("arp",),
        ),
        header_types=(("net", "arp"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="ip-address",
                value_type="string",
                repeated=True,
                shape_kind="ip-address",
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(name="mac-address", value_type="unknown"),
        ),
    )
