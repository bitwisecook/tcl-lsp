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
            "apm_policy_agent_route_domain_selection",
            module="apm",
            object_types=("policy agent route-domain-selection",),
        ),
        header_types=(("apm", "policy agent route-domain-selection"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="route-domain",
                value_type="integer",
                allow_none=True,
                references=("net_route_domain",),
                default="0 (zero)",
            ),
            BigipPropertySpec(
                name="snat",
                value_type="enum",
                allow_none=True,
                enum_values=("automap", "none"),
            ),
            BigipPropertySpec(name="snatpool", value_type="string", allow_none=True),
        ),
    )
