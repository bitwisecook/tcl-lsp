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
            "security_firewall_global_fqdn_policy",
            module="security",
            object_types=("firewall global-fqdn-policy",),
        ),
        header_types=(("security", "firewall global-fqdn-policy"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="dns-resolver", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="refresh-interval", value_type="integer"),
        ),
    )
