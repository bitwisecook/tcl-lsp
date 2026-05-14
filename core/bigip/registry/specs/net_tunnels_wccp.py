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
            "net_tunnels_wccp",
            module="net",
            object_types=("tunnels wccp",),
        ),
        header_types=(("net", "tunnels wccp"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("net_tunnels_wccp",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="rx-csum",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="tx-csum",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="wccp-version", value_type="enum", enum_values=("1", "2")),
        ),
    )
