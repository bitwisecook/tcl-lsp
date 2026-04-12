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
            "sys_ipfix_irules",
            module="sys",
            object_types=("ipfix irules",),
        ),
        header_types=(("sys", "ipfix irules"),),
        properties=(
            BigipPropertySpec(name="reset-stats", value_type="reference", references=("ltm_rule",)),
        ),
    )
