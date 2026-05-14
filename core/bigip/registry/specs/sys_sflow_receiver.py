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
            BigipPropertySpec(name="address", value_type="string"),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="max-datagram-size", value_type="integer"),
            BigipPropertySpec(name="port", value_type="unknown"),
            BigipPropertySpec(name="state", value_type="enum", enum_values=("disabled", "enabled")),
        ),
    )
