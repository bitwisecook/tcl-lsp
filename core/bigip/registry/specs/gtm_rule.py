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
            "gtm_rule",
            module="gtm",
            object_types=("rule",),
        ),
        header_types=(("gtm", "rule"),),
        properties=(
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(name="persist", value_type="enum", enum_values=("true", "false")),
        ),
    )
