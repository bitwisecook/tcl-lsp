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
            "ltm_profile_socks",
            module="ltm",
            object_types=("profile socks",),
        ),
        header_types=(("ltm", "profile socks"),),
        properties=(
            BigipPropertySpec(
                name="default-connect-handling",
                value_type="enum",
                enum_values=("allow", "deny"),
                default="deny",
            ),
            BigipPropertySpec(name="dns-resolver", value_type="unknown", default="dns-resolver"),
            BigipPropertySpec(
                name="ipv6",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no, which will try a IPv4 lookup before a IPv6",
            ),
            BigipPropertySpec(name="protocol-versions", value_type="list"),
            BigipPropertySpec(
                name="route-domain",
                value_type="unknown",
                references=("net_route_domain",),
                default="0",
            ),
            BigipPropertySpec(name="tunnel-name", value_type="unknown", default="socks-tunnel"),
        ),
    )
