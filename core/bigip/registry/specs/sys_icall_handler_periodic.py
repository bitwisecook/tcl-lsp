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
            "sys_icall_handler_periodic",
            module="sys",
            object_types=("icall handler periodic",),
        ),
        header_types=(("sys", "icall handler periodic"),),
        properties=(
            BigipPropertySpec(name="arguments", value_type="string"),
            BigipPropertySpec(name="name", value_type="string", in_sections=("arguments",)),
            BigipPropertySpec(name="value", value_type="string", in_sections=("arguments",)),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="first-occurrence", value_type="string"),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="last-occurrence", value_type="string"),
            BigipPropertySpec(name="script", value_type="string"),
            BigipPropertySpec(name="status", value_type="enum", enum_values=("active", "inactive")),
        ),
    )
