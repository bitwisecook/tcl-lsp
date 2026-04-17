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
            "ltm_dns_nameserver",
            module="ltm",
            object_types=("dns nameserver",),
        ),
        header_types=(("ltm", "dns nameserver"),),
        properties=(
            BigipPropertySpec(
                name="address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(
                name="route-domain",
                value_type="reference",
                allow_none=True,
                references=("net_route_domain",),
            ),
            BigipPropertySpec(name="tsig-key", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
