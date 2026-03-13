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
            "ltm_dns_cache_records_all",
            module="ltm",
            object_types=("dns cache records all",),
        ),
        header_types=(("ltm", "dns cache records all"),),
    )
