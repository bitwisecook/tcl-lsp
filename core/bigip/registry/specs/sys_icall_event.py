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
            "sys_icall_event",
            module="sys",
            object_types=("icall event",),
        ),
        header_types=(("sys", "icall event"),),
        properties=(
            BigipPropertySpec(name="generate", value_type="string"),
            BigipPropertySpec(name="name", value_type="string"),
            BigipPropertySpec(name="context", value_type="string"),
            BigipPropertySpec(name="name", value_type="string", in_sections=("context",)),
            BigipPropertySpec(name="value", value_type="string", in_sections=("context",)),
        ),
    )
