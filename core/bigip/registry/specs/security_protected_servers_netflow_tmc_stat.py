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
            "security_protected_servers_netflow_tmc_stat",
            module="security",
            object_types=("protected-servers netflow-tmc-stat",),
        ),
        header_types=(("security", "protected-servers netflow-tmc-stat"),),
    )
