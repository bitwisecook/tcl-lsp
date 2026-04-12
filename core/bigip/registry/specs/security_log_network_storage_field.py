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
            "security_log_network_storage_field",
            module="security",
            object_types=("log network-storage-field",),
        ),
        header_types=(("security", "log network-storage-field"),),
    )
