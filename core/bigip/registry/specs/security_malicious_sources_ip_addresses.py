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
            "security_malicious_sources_ip_addresses",
            module="security",
            object_types=("malicious-sources ip-addresses",),
        ),
        header_types=(("security", "malicious-sources ip-addresses"),),
    )
