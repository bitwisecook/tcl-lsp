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
            "security_firewall_uuid_default_autogenerate",
            module="security",
            object_types=("firewall uuid-default-autogenerate",),
        ),
        header_types=(("security", "firewall uuid-default-autogenerate"),),
    )
