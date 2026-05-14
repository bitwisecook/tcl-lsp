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
            "sys_diags_ihealth",
            module="sys",
            object_types=("diags ihealth",),
        ),
        header_types=(("sys", "diags ihealth"),),
        properties=(
            BigipPropertySpec(
                name="expiration",
                value_type="unknown",
                default="30, and the maximum is 365",
            ),
            BigipPropertySpec(name="no-ihealth", value_type="unknown"),
            BigipPropertySpec(name="options", value_type="unknown"),
            BigipPropertySpec(name="password", value_type="unknown"),
            BigipPropertySpec(name="user", value_type="unknown"),
        ),
    )
