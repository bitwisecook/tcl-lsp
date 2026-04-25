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
            "ltm_dns_cache_records_msg",
            module="ltm",
            object_types=("dns cache records msg",),
        ),
        header_types=(("ltm", "dns cache records msg"),),
        properties=(
            BigipPropertySpec(name="qname", value_type="string"),
            BigipPropertySpec(name="rcode", value_type="integer"),
            BigipPropertySpec(name="slot", value_type="integer"),
            BigipPropertySpec(name="tmm", value_type="integer"),
        ),
    )
