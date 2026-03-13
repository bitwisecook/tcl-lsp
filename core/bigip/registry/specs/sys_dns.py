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
            "sys_dns",
            module="sys",
            object_types=("dns",),
        ),
        header_types=(("sys", "dns"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="include", value_type="string"),
            BigipPropertySpec(
                name="name-servers",
                value_type="enum",
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="number-of-dots", value_type="integer"),
            BigipPropertySpec(
                name="search", value_type="enum", enum_values=("add", "delete", "replace-all-with")
            ),
        ),
    )
