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
            "security_firewall_config_entity_id",
            module="security",
            object_types=("firewall config-entity-id",),
        ),
        header_types=(("security", "firewall config-entity-id"),),
        properties=(),
    )
