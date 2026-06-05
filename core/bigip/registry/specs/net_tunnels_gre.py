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
            "net_tunnels_gre",
            module="net",
            object_types=("tunnels gre",),
        ),
        header_types=(("net", "tunnels gre"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("net_tunnels_gre",),
                default="gre",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="encapsulation",
                value_type="enum",
                enum_values=("nvgre", "standard"),
                default="standard",
            ),
            BigipPropertySpec(
                name="flooding-type",
                value_type="enum",
                allow_none=True,
                enum_values=("multipoint", "none"),
                default="none",
            ),
            BigipPropertySpec(
                name="rx-csum",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="tx-csum",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
        ),
    )
