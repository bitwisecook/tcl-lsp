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
            "security_firewall_fqdn_entity",
            module="security",
            object_types=("firewall fqdn-entity",),
        ),
        header_types=(("security", "firewall fqdn-entity"),),
        properties=(BigipPropertySpec(name="load", value_type="string"),),
    )
