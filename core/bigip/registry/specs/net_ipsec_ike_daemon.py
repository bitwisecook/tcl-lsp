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
            "net_ipsec_ike_daemon",
            module="net",
            object_types=("ipsec ike-daemon",),
        ),
        header_types=(("net", "ipsec ike-daemon"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="isakmp-natt-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(
                name="isakmp-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(
                name="log-level",
                value_type="enum",
                enum_values=("error", "warning", "notify", "info", "debug", "debug2"),
            ),
            BigipPropertySpec(name="natt-keep-alive", value_type="string"),
            BigipPropertySpec(name="log-publisher", value_type="string"),
        ),
    )
