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
            "ltm_default_node_monitor",
            module="ltm",
            object_types=("default-node-monitor",),
        ),
        header_types=(("ltm", "default-node-monitor"),),
        properties=(
            BigipPropertySpec(name="rule", value_type="reference", references=("ltm_rule",)),
        ),
    )
