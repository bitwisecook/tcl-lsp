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
            "security_flowspec_route_injector_flowspec_advertised_route_info",
            module="security",
            object_types=("flowspec-route-injector flowspec-advertised-route-info",),
        ),
        header_types=(("security", "flowspec-route-injector flowspec-advertised-route-info"),),
    )
