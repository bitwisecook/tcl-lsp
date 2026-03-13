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
            "security_device_device_context",
            module="security",
            object_types=("device device-context",),
        ),
        header_types=(("security", "device device-context"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="nat-policy", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="reset-stats", value_type="string"),
            BigipPropertySpec(
                name="nat-rules", value_type="reference", repeated=True, references=("ltm_rule",)
            ),
            BigipPropertySpec(name="security", value_type="string"),
            BigipPropertySpec(name="nat-policy", value_type="string", in_sections=("security",)),
        ),
    )
