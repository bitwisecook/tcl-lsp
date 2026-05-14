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
            "net_tunnels_lw4o6",
            module="net",
            object_types=("tunnels lw4o6",),
        ),
        header_types=(("net", "tunnels lw4o6"),),
        properties=(
            BigipPropertySpec(
                name="all-protocols-pass",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("net_tunnels_lw4o6",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="lwtbl-file", value_type="string"),
            BigipPropertySpec(name="psid-length", value_type="integer"),
        ),
    )
