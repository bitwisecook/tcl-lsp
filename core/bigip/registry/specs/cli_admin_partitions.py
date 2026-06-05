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
            "cli_admin_partitions",
            module="cli",
            object_types=("admin-partitions",),
        ),
        header_types=(("cli", "admin-partitions"),),
        properties=(BigipPropertySpec(name="update-partition", value_type="reference"),),
    )
