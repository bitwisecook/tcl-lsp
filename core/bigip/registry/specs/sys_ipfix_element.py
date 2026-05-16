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
            "sys_ipfix_element",
            module="sys",
            object_types=("ipfix element",),
        ),
        header_types=(("sys", "ipfix element"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="data-type", value_type="unknown"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="enterprise-id", value_type="integer"),
            BigipPropertySpec(name="id", value_type="integer"),
            BigipPropertySpec(name="size", value_type="integer", default="0"),
        ),
    )
