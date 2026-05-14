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
            "analytics_dns_cache_resolver_report",
            module="analytics",
            object_types=("dns-cache-resolver report",),
        ),
        header_types=(("analytics", "dns-cache-resolver report"),),
        properties=(),
    )
