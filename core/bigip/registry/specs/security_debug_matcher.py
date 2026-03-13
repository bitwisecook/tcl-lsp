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
            "security_debug_matcher",
            module="security",
            object_types=("debug matcher",),
        ),
        header_types=(("security", "debug matcher"),),
        properties=(
            BigipPropertySpec(name="matcher", value_type="string"),
            BigipPropertySpec(name="drop-redirect", value_type="string", in_sections=("matcher",)),
            BigipPropertySpec(
                name="drop-redirect-mode", value_type="string", in_sections=("drop-redirect",)
            ),
        ),
    )
