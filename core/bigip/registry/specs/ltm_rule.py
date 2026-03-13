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
            "ltm_rule",
            module="ltm",
            object_types=("rule",),
        ),
        header_types=(("ltm", "rule"),),
        properties=(
            BigipPropertySpec(name="generate", value_type="reference", references=("ltm_rule",)),
            BigipPropertySpec(name="when", value_type="string"),
            BigipPropertySpec(name="timing", value_type="string"),
            BigipPropertySpec(name="check", value_type="string"),
        ),
    )
