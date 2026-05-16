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
                shape_kind="ip-address",
                default="127",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(name="port", value_type="integer", default="53"),
            BigipPropertySpec(
                name="route-domain",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
                references=("net_route_domain",),
                default="the default route domain",
            ),
            BigipPropertySpec(
                name="tsig-key",
                value_type="reference",
                allow_none=True,
                enum_values=("none",),
                references=("ltm_dns_tsig_key",),
            ),
        ),
    )
