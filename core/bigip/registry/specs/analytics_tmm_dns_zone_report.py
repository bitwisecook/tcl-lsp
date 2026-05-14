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
            "analytics_tmm_dns_zone_report",
            module="analytics",
            object_types=("tmm-dns-zone report",),
        ),
        header_types=(("analytics", "tmm-dns-zone report"),),
        properties=(),
    )
