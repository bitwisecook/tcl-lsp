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
            "sys_ecm_config",
            module="sys",
            object_types=("ecm config",),
        ),
        header_types=(("sys", "ecm config"),),
        properties=(
            BigipPropertySpec(name="seed-ip", value_type="string"),
            BigipPropertySpec(name="dns-resolver", value_type="string"),
            BigipPropertySpec(name="auth", value_type="string"),
            BigipPropertySpec(name="status", value_type="string"),
        ),
    )
