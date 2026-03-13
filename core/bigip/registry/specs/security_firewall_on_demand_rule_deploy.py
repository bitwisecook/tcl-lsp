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
            "security_firewall_on_demand_rule_deploy",
            module="security",
            object_types=("firewall on-demand-rule-deploy",),
        ),
        header_types=(("security", "firewall on-demand-rule-deploy"),),
    )
