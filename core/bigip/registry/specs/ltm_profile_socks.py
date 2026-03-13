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
            BigipPropertySpec(name="protocol-versions", value_type="string"),
            BigipPropertySpec(name="dns-resolver", value_type="string"),
            BigipPropertySpec(name="ipv6", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(name="tunnel-name", value_type="string"),
            BigipPropertySpec(
                name="route-domain", value_type="reference", references=("net_route_domain",)
            ),
            BigipPropertySpec(
                name="default-connect-handling", value_type="enum", enum_values=("deny", "allow")
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
