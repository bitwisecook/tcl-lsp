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
            "net_tunnels_ppp",
            module="net",
            object_types=("tunnels ppp",),
        ),
        header_types=(("net", "tunnels ppp"),),
        properties=(
            BigipPropertySpec(name="defaults-from", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="lcp-echo-failure", value_type="integer"),
            BigipPropertySpec(name="lcp-echo-interval", value_type="integer"),
            BigipPropertySpec(name="vj", value_type="enum", enum_values=("disabled", "enabled")),
        ),
    )
