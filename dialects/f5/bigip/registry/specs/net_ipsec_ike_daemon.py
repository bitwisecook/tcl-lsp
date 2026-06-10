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
            BigipPropertySpec(name="isakmp-natt-port", value_type="unknown"),
            BigipPropertySpec(name="isakmp-port", value_type="unknown"),
            BigipPropertySpec(
                name="log-level",
                value_type="enum",
                enum_values=("debug", "debug2", "error", "info", "notify", "warning"),
            ),
            BigipPropertySpec(name="log-publisher", value_type="string"),
            BigipPropertySpec(name="natt-keep-alive", value_type="unknown", default="20 seconds"),
        ),
    )
