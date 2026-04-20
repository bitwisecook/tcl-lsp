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
            "ltm_dns_cache_records_nameserver",
            module="ltm",
            object_types=("dns cache records nameserver",),
        ),
        header_types=(("ltm", "dns cache records nameserver"),),
        properties=(
            BigipPropertySpec(
                name="address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="has-edns", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="has-lame", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(name="rtt-range", value_type="string"),
            BigipPropertySpec(name="slot", value_type="integer"),
            BigipPropertySpec(name="tmm", value_type="integer"),
            BigipPropertySpec(name="ttl-range", value_type="string"),
            BigipPropertySpec(name="zone-name", value_type="string"),
        ),
    )
