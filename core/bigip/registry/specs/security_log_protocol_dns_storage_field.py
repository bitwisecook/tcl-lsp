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
            "security_log_protocol_dns_storage_field",
            module="security",
            object_types=("log protocol-dns-storage-field",),
        ),
        header_types=(("security", "log protocol-dns-storage-field"),),
    )
