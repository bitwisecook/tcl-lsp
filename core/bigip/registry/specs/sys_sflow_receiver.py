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
            "sys_sflow_receiver",
            module="sys",
            object_types=("sflow receiver",),
        ),
        header_types=(("sys", "sflow receiver"),),
        properties=(
            BigipPropertySpec(
                name="address",
                value_type="string",
                required=True,
                shape_kind="ip-address",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="max-datagram-size", value_type="integer", default="1400"),
            BigipPropertySpec(
                name="port",
                value_type="unknown",
                default="the standard sFlow port, 6343",
            ),
            BigipPropertySpec(
                name="state",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
        ),
    )
