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
            "net_routing_access_list",
            module="net",
            object_types=("routing access-list",),
        ),
        header_types=(("net", "routing access-list"),),
        properties=(
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="route-domain",
                value_type="reference",
                allow_none=True,
                references=("net_route_domain",),
            ),
            BigipPropertySpec(
                name="entries",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="action", value_type="boolean", in_sections=("entries",), allow_none=True
            ),
            BigipPropertySpec(name="destination", value_type="string", in_sections=("entries",)),
            BigipPropertySpec(
                name="exact-match",
                value_type="enum",
                in_sections=("entries",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="source", value_type="string", in_sections=("entries",)),
        ),
    )
