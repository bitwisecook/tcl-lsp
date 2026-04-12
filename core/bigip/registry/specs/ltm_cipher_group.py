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
            "ltm_cipher_group",
            module="ltm",
            object_types=("cipher group",),
        ),
        header_types=(("ltm", "cipher group"),),
        properties=(
            BigipPropertySpec(
                name="allow", value_type="reference", repeated=True, references=("ltm_rule",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="exclude", value_type="reference", repeated=True, references=("ltm_rule",)
            ),
            BigipPropertySpec(
                name="require", value_type="reference", repeated=True, references=("ltm_rule",)
            ),
        ),
    )
