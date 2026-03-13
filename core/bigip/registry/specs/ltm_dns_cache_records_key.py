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
            "ltm_dns_cache_records_key",
            module="ltm",
            object_types=("dns cache records key",),
        ),
        header_types=(("ltm", "dns cache records key"),),
        properties=(
            BigipPropertySpec(name="owner", value_type="string"),
            BigipPropertySpec(name="slot", value_type="integer"),
            BigipPropertySpec(name="tmm", value_type="integer"),
        ),
    )
