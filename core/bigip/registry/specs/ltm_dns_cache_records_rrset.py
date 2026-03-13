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
            "ltm_dns_cache_records_rrset",
            module="ltm",
            object_types=("dns cache records rrset",),
        ),
        header_types=(("ltm", "dns cache records rrset"),),
        properties=(
            BigipPropertySpec(
                name="class", value_type="enum", enum_values=("in", "ch", "hs", "any")
            ),
            BigipPropertySpec(name="owner", value_type="string"),
            BigipPropertySpec(name="slot", value_type="integer"),
            BigipPropertySpec(name="tmm", value_type="integer"),
            BigipPropertySpec(name="ttl-range", value_type="integer"),
            BigipPropertySpec(name="type", value_type="list", repeated=True),
        ),
    )
