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
            "sys_application_custom_stat",
            module="sys",
            object_types=("application custom-stat",),
        ),
        header_types=(("sys", "application custom-stat"),),
        properties=(
            BigipPropertySpec(name="keyspace", value_type="string"),
            BigipPropertySpec(name="formula", value_type="string"),
            BigipPropertySpec(name="measure", value_type="string"),
            BigipPropertySpec(name="rate", value_type="string"),
        ),
    )
